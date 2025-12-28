"""CRA trace command - Query and stream TRACE events."""

import asyncio
from uuid import UUID

import httpx
import typer
from rich.console import Console
from rich.json import JSON

from cra.cli.config import get_config

app = typer.Typer(help="Query and stream TRACE events")
console = Console()


@app.command("tail")
def tail(
    trace_id: str = typer.Option(
        ...,
        "--trace-id",
        "-t",
        help="Trace ID to stream",
    ),
    follow: bool = typer.Option(
        False,
        "--follow",
        "-f",
        help="Follow (stream) new events",
    ),
    severity: str = typer.Option(
        None,
        "--severity",
        "-s",
        help="Filter by severity (debug, info, warn, error)",
    ),
    event_type: str = typer.Option(
        None,
        "--event-type",
        "-e",
        help="Filter by event type prefix",
    ),
    raw: bool = typer.Option(
        False,
        "--raw",
        help="Output raw JSONL only",
    ),
) -> None:
    """Stream or query TRACE events.

    Without --follow, fetches existing events.
    With --follow, streams events in real-time via SSE.

    Events are output as JSONL for machine processing.
    """
    config = get_config()
    runtime_url = config.runtime.url

    try:
        trace_uuid = UUID(trace_id)
    except ValueError:
        console.print(f"[red]Invalid trace ID: {trace_id}[/red]")
        raise typer.Exit(1)

    if follow:
        # Stream mode
        asyncio.run(_stream_events(runtime_url, trace_uuid, raw))
    else:
        # Query mode
        _query_events(runtime_url, trace_uuid, severity, event_type, raw)


def _query_events(
    runtime_url: str,
    trace_id: UUID,
    severity: str | None,
    event_type: str | None,
    raw: bool,
) -> None:
    """Query existing events."""
    params = {}
    if severity:
        params["severity"] = severity
    if event_type:
        params["event_type"] = event_type

    try:
        response = httpx.get(
            f"{runtime_url}/v1/traces/{trace_id}/events",
            params=params,
            timeout=30.0,
        )
        response.raise_for_status()
        data = response.json()

        events = data.get("events", [])
        total = data.get("total_count", 0)

        if not raw:
            console.print(f"[dim]Showing {len(events)} of {total} events[/dim]")
            console.print()

        for event in events:
            if raw:
                console.print_json(data=event)
            else:
                console.print(JSON.from_data(event))

        if data.get("has_more"):
            console.print()
            console.print("[dim]More events available. Use --limit and --offset for pagination.[/dim]")

    except httpx.ConnectError:
        console.print(f"[red]Cannot connect to runtime at {runtime_url}[/red]")
        raise typer.Exit(1)
    except httpx.HTTPStatusError as e:
        console.print(f"[red]HTTP error: {e.response.status_code}[/red]")
        raise typer.Exit(1)


async def _stream_events(runtime_url: str, trace_id: UUID, raw: bool) -> None:
    """Stream events via SSE."""
    import httpx_sse

    if not raw:
        console.print(f"[dim]Streaming events for trace {trace_id}...[/dim]")
        console.print(f"[dim]Press Ctrl+C to stop.[/dim]")
        console.print()

    try:
        async with httpx.AsyncClient() as client:
            async with httpx_sse.aconnect_sse(
                client,
                "GET",
                f"{runtime_url}/v1/traces/{trace_id}/stream",
            ) as event_source:
                async for sse in event_source.aiter_sse():
                    if sse.data:
                        if raw:
                            console.print(sse.data)
                        else:
                            import json
                            event = json.loads(sse.data)
                            console.print(JSON.from_data(event))
    except KeyboardInterrupt:
        if not raw:
            console.print()
            console.print("[dim]Stopped.[/dim]")
    except httpx.ConnectError:
        console.print(f"[red]Cannot connect to runtime at {runtime_url}[/red]")
        raise typer.Exit(1)


@app.command("events")
def events(
    trace_id: str = typer.Option(
        ...,
        "--trace-id",
        "-t",
        help="Trace ID to query",
    ),
    limit: int = typer.Option(
        100,
        "--limit",
        "-l",
        help="Maximum events to return",
    ),
    offset: int = typer.Option(
        0,
        "--offset",
        "-o",
        help="Offset for pagination",
    ),
    severity: str = typer.Option(
        None,
        "--severity",
        "-s",
        help="Filter by severity",
    ),
    event_type: str = typer.Option(
        None,
        "--event-type",
        "-e",
        help="Filter by event type prefix",
    ),
) -> None:
    """Query TRACE events with pagination.

    Returns events for a trace with optional filtering.
    """
    config = get_config()
    runtime_url = config.runtime.url

    try:
        trace_uuid = UUID(trace_id)
    except ValueError:
        console.print(f"[red]Invalid trace ID: {trace_id}[/red]")
        raise typer.Exit(1)

    params = {"limit": limit, "offset": offset}
    if severity:
        params["severity"] = severity
    if event_type:
        params["event_type"] = event_type

    try:
        response = httpx.get(
            f"{runtime_url}/v1/traces/{trace_uuid}/events",
            params=params,
            timeout=30.0,
        )
        response.raise_for_status()
        data = response.json()

        console.print(JSON.from_data(data))

    except httpx.ConnectError:
        console.print(f"[red]Cannot connect to runtime at {runtime_url}[/red]")
        raise typer.Exit(1)
    except httpx.HTTPStatusError as e:
        console.print(f"[red]HTTP error: {e.response.status_code}[/red]")
        raise typer.Exit(1)
