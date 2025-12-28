"""CRA execute command - Execute granted actions."""

from datetime import datetime
from uuid import UUID, uuid4

import httpx
import typer
from rich.console import Console
from rich.json import JSON

from cra.cli.config import get_config

app = typer.Typer(help="Execute granted actions")
console = Console()


@app.callback(invoke_without_command=True)
def execute(
    ctx: typer.Context,
    session_id: str = typer.Option(
        ...,
        "--session",
        "-s",
        help="Session ID",
    ),
    resolution_id: str = typer.Option(
        ...,
        "--resolution",
        "-r",
        help="Resolution ID from CARP resolve",
    ),
    action_id: str = typer.Option(
        ...,
        "--action",
        "-a",
        help="Action ID to execute",
    ),
    parameters: str = typer.Option(
        "{}",
        "--params",
        "-p",
        help="JSON parameters for the action",
    ),
    trace_id: str = typer.Option(
        None,
        "--trace-id",
        "-t",
        help="Trace ID (uses session trace if not provided)",
    ),
) -> None:
    """Execute a granted action.

    The action must have been granted in a prior CARP resolution.
    If the action requires approval, it must be approved first.

    TRACE events are emitted for the execution.
    """
    import json

    config = get_config()
    runtime_url = config.runtime.url

    try:
        params = json.loads(parameters)
    except json.JSONDecodeError as e:
        console.print(f"[red]Invalid JSON parameters: {e}[/red]")
        raise typer.Exit(1)

    try:
        session_uuid = UUID(session_id)
        resolution_uuid = UUID(resolution_id)
        trace_uuid = UUID(trace_id) if trace_id else uuid4()
    except ValueError as e:
        console.print(f"[red]Invalid UUID: {e}[/red]")
        raise typer.Exit(1)

    request = {
        "session_id": str(session_uuid),
        "resolution_id": str(resolution_uuid),
        "action_id": action_id,
        "parameters": params,
        "trace_id": str(trace_uuid),
        "span_id": str(uuid4()),
    }

    try:
        response = httpx.post(
            f"{runtime_url}/v1/carp/execute",
            json=request,
            timeout=config.runtime.timeout_ms / 1000,
        )

        if response.status_code == 403:
            console.print("[yellow]Action requires approval.[/yellow]")
            console.print("Use 'cra approve' to approve the action first.")
            raise typer.Exit(1)

        if response.status_code == 404:
            console.print("[red]Action grant not found.[/red]")
            console.print("Ensure the action was granted in the resolution.")
            raise typer.Exit(1)

        if response.status_code == 410:
            console.print("[red]Action grant has expired.[/red]")
            console.print("Request a new resolution.")
            raise typer.Exit(1)

        response.raise_for_status()
        data = response.json()

        console.print()
        console.print("[bold]Execution Result:[/bold]")
        console.print(JSON.from_data(data))

    except httpx.ConnectError:
        console.print(f"[red]Cannot connect to runtime at {runtime_url}[/red]")
        raise typer.Exit(1)
    except httpx.HTTPStatusError as e:
        console.print(f"[red]HTTP error: {e.response.status_code}[/red]")
        console.print_json(data=e.response.json())
        raise typer.Exit(1)


@app.command("approve")
def approve(
    grant_id: str = typer.Option(
        ...,
        "--grant",
        "-g",
        help="Grant ID to approve",
    ),
    session_id: str = typer.Option(
        ...,
        "--session",
        "-s",
        help="Session ID",
    ),
    trace_id: str = typer.Option(
        ...,
        "--trace-id",
        "-t",
        help="Trace ID",
    ),
    approved_by: str = typer.Option(
        "cli-user",
        "--by",
        "-b",
        help="Who is approving",
    ),
) -> None:
    """Approve a pending action.

    Actions with requires_approval=true must be approved before execution.
    """
    config = get_config()
    runtime_url = config.runtime.url

    try:
        grant_uuid = UUID(grant_id)
        session_uuid = UUID(session_id)
        trace_uuid = UUID(trace_id)
    except ValueError as e:
        console.print(f"[red]Invalid UUID: {e}[/red]")
        raise typer.Exit(1)

    request = {
        "grant_id": str(grant_uuid),
        "session_id": str(session_uuid),
        "trace_id": str(trace_uuid),
        "approved_by": approved_by,
    }

    try:
        response = httpx.post(
            f"{runtime_url}/v1/carp/actions/{grant_id}/approve",
            json=request,
            timeout=config.runtime.timeout_ms / 1000,
        )
        response.raise_for_status()
        data = response.json()

        console.print()
        console.print("[green]Action approved.[/green]")
        console.print(JSON.from_data(data))

    except httpx.ConnectError:
        console.print(f"[red]Cannot connect to runtime at {runtime_url}[/red]")
        raise typer.Exit(1)
    except httpx.HTTPStatusError as e:
        console.print(f"[red]HTTP error: {e.response.status_code}[/red]")
        raise typer.Exit(1)


@app.command("reject")
def reject(
    grant_id: str = typer.Option(
        ...,
        "--grant",
        "-g",
        help="Grant ID to reject",
    ),
    session_id: str = typer.Option(
        ...,
        "--session",
        "-s",
        help="Session ID",
    ),
    trace_id: str = typer.Option(
        ...,
        "--trace-id",
        "-t",
        help="Trace ID",
    ),
    rejected_by: str = typer.Option(
        "cli-user",
        "--by",
        "-b",
        help="Who is rejecting",
    ),
    reason: str = typer.Option(
        ...,
        "--reason",
        "-r",
        help="Reason for rejection",
    ),
) -> None:
    """Reject a pending action.

    Rejected actions cannot be executed.
    """
    config = get_config()
    runtime_url = config.runtime.url

    try:
        grant_uuid = UUID(grant_id)
        session_uuid = UUID(session_id)
        trace_uuid = UUID(trace_id)
    except ValueError as e:
        console.print(f"[red]Invalid UUID: {e}[/red]")
        raise typer.Exit(1)

    request = {
        "grant_id": str(grant_uuid),
        "session_id": str(session_uuid),
        "trace_id": str(trace_uuid),
        "rejected_by": rejected_by,
        "reason": reason,
    }

    try:
        response = httpx.post(
            f"{runtime_url}/v1/carp/actions/{grant_id}/reject",
            json=request,
            timeout=config.runtime.timeout_ms / 1000,
        )
        response.raise_for_status()
        data = response.json()

        console.print()
        console.print("[yellow]Action rejected.[/yellow]")
        console.print(JSON.from_data(data))

    except httpx.ConnectError:
        console.print(f"[red]Cannot connect to runtime at {runtime_url}[/red]")
        raise typer.Exit(1)
    except httpx.HTTPStatusError as e:
        console.print(f"[red]HTTP error: {e.response.status_code}[/red]")
        raise typer.Exit(1)


@app.command("pending")
def pending() -> None:
    """List pending action approvals."""
    config = get_config()
    runtime_url = config.runtime.url

    try:
        response = httpx.get(
            f"{runtime_url}/v1/carp/actions/pending",
            timeout=config.runtime.timeout_ms / 1000,
        )
        response.raise_for_status()
        data = response.json()

        if data["count"] == 0:
            console.print("[dim]No pending approvals.[/dim]")
            return

        console.print()
        console.print(f"[bold]Pending Approvals ({data['count']}):[/bold]")
        console.print()

        for approval in data["approvals"]:
            console.print(f"  Grant ID: {approval['grant_id']}")
            console.print(f"  Action:   {approval['action_id']}")
            console.print(f"  Risk:     {approval['risk_tier']}")
            console.print(f"  Reason:   {approval['reason']}")
            console.print()

    except httpx.ConnectError:
        console.print(f"[red]Cannot connect to runtime at {runtime_url}[/red]")
        raise typer.Exit(1)
