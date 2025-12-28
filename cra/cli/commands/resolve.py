"""CRA resolve command - Resolve context and actions."""

from datetime import datetime
from uuid import uuid4

import httpx
import typer
from rich.console import Console
from rich.json import JSON

from cra.version import CARP_VERSION
from cra.cli.config import get_config

app = typer.Typer(help="Resolve context and actions via CARP")
console = Console()


@app.callback(invoke_without_command=True)
def resolve(
    ctx: typer.Context,
    goal: str = typer.Option(
        ...,
        "--goal",
        "-g",
        help="Task goal to resolve",
    ),
    risk_tier: str = typer.Option(
        "medium",
        "--risk-tier",
        "-r",
        help="Risk tier (low, medium, high)",
    ),
    session_id: str = typer.Option(
        None,
        "--session",
        "-s",
        help="Existing session ID to use",
    ),
    raw: bool = typer.Option(
        False,
        "--raw",
        help="Output raw JSONL only",
    ),
) -> None:
    """Resolve context and actions for a task.

    Calls the CRA runtime to get a Resolution Bundle containing:
    - Context blocks (TTL-bounded)
    - Allowed actions
    - Deny rules
    - Next steps

    TRACE events are streamed as raw JSONL.
    """
    config = get_config()
    runtime_url = config.runtime.url

    # Create a session if none provided
    if session_id is None:
        try:
            response = httpx.post(
                f"{runtime_url}/v1/sessions",
                json={
                    "principal": {"type": "user", "id": "cli-user"},
                    "scopes": ["carp.resolve"],
                    "ttl_seconds": 3600,
                },
                timeout=config.runtime.timeout_ms / 1000,
            )
            response.raise_for_status()
            session_data = response.json()
            session_id = session_data["session_id"]
            trace_id = session_data["trace_id"]

            if not raw:
                console.print(f"[dim]Created session: {session_id}[/dim]")
        except httpx.HTTPError as e:
            console.print(f"[red]Failed to create session: {e}[/red]")
            raise typer.Exit(1)
    else:
        trace_id = str(uuid4())

    # Build CARP request
    request_id = str(uuid4())
    span_id = str(uuid4())

    carp_request = {
        "carp_version": CARP_VERSION,
        "type": "carp.request",
        "id": request_id,
        "time": datetime.utcnow().isoformat() + "Z",
        "session": {
            "session_id": session_id,
            "principal": {"type": "user", "id": "cli-user"},
            "scopes": ["carp.resolve"],
        },
        "atlas": None,
        "payload": {
            "operation": "resolve",
            "task": {
                "goal": goal,
                "inputs": [],
                "constraints": [],
                "target_platforms": ["openai.tools", "anthropic.skills"],
                "risk_tier": risk_tier,
            },
            "environment": {
                "project_root": None,
                "os": None,
                "cli_capabilities": ["bash", "python", "git"],
                "network_policy": "open",
            },
            "preferences": {
                "verbosity": "standard",
                "format": ["json", "markdown"],
                "explainability": "standard",
            },
        },
        "trace": {
            "trace_id": trace_id,
            "span_id": span_id,
            "parent_span_id": None,
        },
    }

    try:
        # Stream TRACE events
        if not raw:
            console.print()
            console.print("[bold]TRACE Events:[/bold]")

        # Make the resolve request
        response = httpx.post(
            f"{runtime_url}/v1/carp/resolve",
            json=carp_request,
            timeout=config.runtime.timeout_ms / 1000,
        )
        response.raise_for_status()
        resolution_data = response.json()

        # Fetch trace events
        trace_response = httpx.get(
            f"{runtime_url}/v1/traces/{trace_id}/events",
            timeout=config.runtime.timeout_ms / 1000,
        )
        trace_response.raise_for_status()
        trace_data = trace_response.json()

        # Output TRACE events as JSONL
        for event in trace_data.get("events", []):
            event_json = JSON.from_data(event)
            if raw:
                console.print_json(data=event)
            else:
                console.print(event_json)

        if not raw:
            console.print()
            console.print("[bold]Resolution Bundle:[/bold]")
            console.print()

        # Output resolution
        resolution = resolution_data.get("payload", {}).get("resolution", {})
        if raw:
            console.print_json(data=resolution)
        else:
            console.print(JSON.from_data(resolution))

        console.print()

    except httpx.ConnectError:
        console.print(f"[red]Cannot connect to runtime at {runtime_url}[/red]")
        console.print("[dim]Start the runtime with: python -m cra.runtime.server[/dim]")
        raise typer.Exit(1)
    except httpx.HTTPStatusError as e:
        console.print(f"[red]HTTP error: {e.response.status_code}[/red]")
        console.print_json(data=e.response.json())
        raise typer.Exit(1)
