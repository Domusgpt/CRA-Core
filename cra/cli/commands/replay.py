"""CRA replay command - Trace replay and golden trace management."""

import json
from pathlib import Path
from uuid import UUID

import httpx
import typer
from rich.console import Console
from rich.table import Table

from cra.cli.config import get_config
from cra.core.replay import TraceReplayer, ReplayManifest

app = typer.Typer(help="Replay and compare traces")
console = Console()


@app.command("compare")
def compare(
    manifest_path: Path = typer.Argument(
        ...,
        help="Path to replay manifest file",
    ),
    trace_id: str = typer.Option(
        None,
        "--trace-id",
        "-t",
        help="Trace ID to compare against (uses manifest trace_id if not provided)",
    ),
) -> None:
    """Compare a trace against a golden manifest.

    Loads the manifest and compares expected events against actual events.
    Handles nondeterminism according to manifest rules.
    """
    if not manifest_path.exists():
        console.print(f"[red]Manifest not found: {manifest_path}[/red]")
        raise typer.Exit(1)

    config = get_config()
    runtime_url = config.runtime.url

    # Load manifest
    replayer = TraceReplayer()
    try:
        manifest = replayer.load_manifest(manifest_path)
    except Exception as e:
        console.print(f"[red]Failed to load manifest: {e}[/red]")
        raise typer.Exit(1)

    # Use provided trace_id or manifest's
    actual_trace_id = UUID(trace_id) if trace_id else manifest.trace_id

    console.print(f"[dim]Comparing trace {actual_trace_id}[/dim]")
    console.print(f"[dim]Against manifest: {manifest.name or manifest_path.name}[/dim]")
    console.print()

    # Fetch actual events
    try:
        response = httpx.get(
            f"{runtime_url}/v1/traces/{actual_trace_id}/events",
            params={"limit": 1000},
            timeout=config.runtime.timeout_ms / 1000,
        )
        response.raise_for_status()
        data = response.json()
        actual_events = data.get("events", [])
    except httpx.ConnectError:
        console.print(f"[red]Cannot connect to runtime at {runtime_url}[/red]")
        raise typer.Exit(1)
    except httpx.HTTPStatusError as e:
        console.print(f"[red]Failed to fetch trace: {e.response.status_code}[/red]")
        raise typer.Exit(1)

    # Set nondeterminism rules from manifest
    replayer.set_rules(manifest.nondeterminism)

    # Convert actual events to TraceEvent objects for comparison
    from cra.core.trace import TraceEvent
    actual_trace_events = []
    for event_dict in actual_events:
        try:
            actual_trace_events.append(TraceEvent.model_validate(event_dict))
        except Exception:
            # Skip events that don't validate
            pass

    # Compare
    result = replayer.compare(
        expected=manifest.expected_events,
        actual=actual_trace_events,
        manifest_name=manifest.name or str(manifest_path),
        trace_id=actual_trace_id,
    )

    # Display results
    if result.success:
        console.print("[green]PASS[/green] - All events match")
        console.print()
        console.print(f"  Events matched: {result.matched_events}/{result.expected_count}")
        console.print(f"  Duration: {result.duration_ms}ms")
    else:
        console.print("[red]FAIL[/red] - Events differ")
        console.print()
        console.print(f"  Expected: {result.expected_count} events")
        console.print(f"  Actual:   {result.actual_count} events")
        console.print(f"  Matched:  {result.matched_events}")
        console.print()

        if result.differences:
            console.print("[bold]Differences:[/bold]")
            table = Table(show_header=True)
            table.add_column("Event #")
            table.add_column("Field")
            table.add_column("Expected")
            table.add_column("Actual")
            table.add_column("Severity")

            for diff in result.differences[:20]:  # Limit to 20
                table.add_row(
                    str(diff.event_index),
                    diff.field,
                    str(diff.expected)[:50],
                    str(diff.actual)[:50],
                    diff.severity,
                )

            console.print(table)

            if len(result.differences) > 20:
                console.print(f"[dim]... and {len(result.differences) - 20} more differences[/dim]")

    if result.skipped_fields:
        console.print()
        console.print(f"[dim]Skipped fields: {', '.join(result.skipped_fields)}[/dim]")

    raise typer.Exit(0 if result.success else 1)


@app.command("create")
def create(
    trace_id: str = typer.Option(
        ...,
        "--trace-id",
        "-t",
        help="Trace ID to create manifest from",
    ),
    output: Path = typer.Option(
        ...,
        "--output",
        "-o",
        help="Output path for manifest file",
    ),
    name: str = typer.Option(
        "",
        "--name",
        "-n",
        help="Name for the manifest",
    ),
    description: str = typer.Option(
        "",
        "--description",
        "-d",
        help="Description for the manifest",
    ),
    tags: str = typer.Option(
        "",
        "--tags",
        help="Comma-separated tags",
    ),
) -> None:
    """Create a golden trace manifest from an existing trace.

    Fetches all events for a trace and saves them as a replay manifest.
    """
    config = get_config()
    runtime_url = config.runtime.url

    try:
        trace_uuid = UUID(trace_id)
    except ValueError:
        console.print(f"[red]Invalid trace ID: {trace_id}[/red]")
        raise typer.Exit(1)

    # Fetch events
    try:
        response = httpx.get(
            f"{runtime_url}/v1/traces/{trace_uuid}/events",
            params={"limit": 1000},
            timeout=config.runtime.timeout_ms / 1000,
        )
        response.raise_for_status()
        data = response.json()
        events = data.get("events", [])
    except httpx.ConnectError:
        console.print(f"[red]Cannot connect to runtime at {runtime_url}[/red]")
        raise typer.Exit(1)
    except httpx.HTTPStatusError as e:
        console.print(f"[red]Failed to fetch trace: {e.response.status_code}[/red]")
        raise typer.Exit(1)

    if not events:
        console.print("[yellow]No events found for trace.[/yellow]")
        raise typer.Exit(1)

    # Convert to TraceEvent objects
    from cra.core.trace import TraceEvent
    trace_events = []
    for event_dict in events:
        try:
            trace_events.append(TraceEvent.model_validate(event_dict))
        except Exception as e:
            console.print(f"[yellow]Skipping invalid event: {e}[/yellow]")

    if not trace_events:
        console.print("[red]No valid events found.[/red]")
        raise typer.Exit(1)

    # Create manifest
    replayer = TraceReplayer()
    tag_list = [t.strip() for t in tags.split(",") if t.strip()] if tags else []

    manifest = replayer.create_manifest(
        events=trace_events,
        name=name or f"Golden trace {trace_id[:8]}",
        description=description,
        tags=tag_list,
    )

    # Save manifest
    try:
        replayer.save_manifest(manifest, output)
        console.print(f"[green]Created manifest: {output}[/green]")
        console.print(f"  Events: {len(trace_events)}")
        console.print(f"  Trace ID: {trace_uuid}")
    except Exception as e:
        console.print(f"[red]Failed to save manifest: {e}[/red]")
        raise typer.Exit(1)


@app.command("validate")
def validate(
    manifest_path: Path = typer.Argument(
        ...,
        help="Path to manifest file to validate",
    ),
) -> None:
    """Validate a replay manifest file.

    Checks that the manifest is well-formed and all events are valid.
    """
    if not manifest_path.exists():
        console.print(f"[red]File not found: {manifest_path}[/red]")
        raise typer.Exit(1)

    replayer = TraceReplayer()

    try:
        manifest = replayer.load_manifest(manifest_path)
    except Exception as e:
        console.print(f"[red]Invalid manifest: {e}[/red]")
        raise typer.Exit(1)

    console.print("[green]Manifest is valid.[/green]")
    console.print()
    console.print(f"  Version:     {manifest.manifest_version}")
    console.print(f"  Name:        {manifest.name or '(unnamed)'}")
    console.print(f"  Trace ID:    {manifest.trace_id}")
    console.print(f"  Events:      {len(manifest.expected_events)}")
    console.print(f"  Rules:       {len(manifest.nondeterminism)}")
    console.print(f"  Tags:        {', '.join(manifest.tags) if manifest.tags else '(none)'}")

    # Validate each event
    from cra.core.trace import TraceEvent
    valid_count = 0
    invalid_count = 0

    for i, event in enumerate(manifest.expected_events):
        try:
            TraceEvent.model_validate(event)
            valid_count += 1
        except Exception as e:
            invalid_count += 1
            if invalid_count <= 5:
                console.print(f"  [yellow]Event {i} invalid: {e}[/yellow]")

    console.print()
    console.print(f"  Valid events:   {valid_count}")
    if invalid_count:
        console.print(f"  [yellow]Invalid events: {invalid_count}[/yellow]")
