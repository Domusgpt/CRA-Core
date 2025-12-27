"""CRA atlas command - Manage Atlases."""

from pathlib import Path

import httpx
import typer
from rich.console import Console
from rich.json import JSON
from rich.table import Table

from cra.cli.config import get_config

app = typer.Typer(help="Manage Atlases")
console = Console()


@app.command("list")
def list_atlases() -> None:
    """List all registered Atlases."""
    config = get_config()
    runtime_url = config.runtime.url

    try:
        response = httpx.get(
            f"{runtime_url}/v1/atlases",
            timeout=config.runtime.timeout_ms / 1000,
        )
        response.raise_for_status()
        data = response.json()

        if data["count"] == 0:
            console.print("[dim]No Atlases registered.[/dim]")
            console.print()
            console.print("Load an Atlas with: cra atlas load <path>")
            return

        table = Table(title=f"Registered Atlases ({data['count']})")
        table.add_column("ID", style="cyan")
        table.add_column("Version")
        table.add_column("Name")
        table.add_column("Capabilities")
        table.add_column("Adapters")

        for atlas in data["atlases"]:
            table.add_row(
                atlas["id"],
                atlas["version"],
                atlas["name"],
                ", ".join(atlas["capabilities"][:3]),
                ", ".join(atlas["adapters"]),
            )

        console.print(table)

    except httpx.ConnectError:
        console.print(f"[red]Cannot connect to runtime at {runtime_url}[/red]")
        raise typer.Exit(1)


@app.command("load")
def load_atlas(
    path: Path = typer.Argument(
        ...,
        help="Path to Atlas directory",
    ),
) -> None:
    """Load an Atlas from a local path.

    The path should contain an atlas.json manifest file.
    """
    if not path.exists():
        console.print(f"[red]Path not found: {path}[/red]")
        raise typer.Exit(1)

    manifest_path = path / "atlas.json"
    if not manifest_path.exists():
        console.print(f"[red]atlas.json not found in {path}[/red]")
        raise typer.Exit(1)

    config = get_config()
    runtime_url = config.runtime.url

    try:
        response = httpx.post(
            f"{runtime_url}/v1/atlases/load",
            json={"path": str(path.absolute())},
            timeout=config.runtime.timeout_ms / 1000,
        )
        response.raise_for_status()
        data = response.json()

        if data["success"]:
            console.print("[green]Atlas loaded successfully![/green]")
            console.print()
            atlas = data["atlas"]
            console.print(f"  ID:           {atlas['id']}")
            console.print(f"  Version:      {atlas['version']}")
            console.print(f"  Name:         {atlas['name']}")
            console.print(f"  Capabilities: {', '.join(atlas['capabilities'])}")
            console.print(f"  Adapters:     {', '.join(atlas['adapters'])}")
        else:
            console.print(f"[red]Failed to load Atlas: {data['error']}[/red]")
            raise typer.Exit(1)

    except httpx.ConnectError:
        console.print(f"[red]Cannot connect to runtime at {runtime_url}[/red]")
        raise typer.Exit(1)


@app.command("info")
def atlas_info(
    atlas_id: str = typer.Argument(
        ...,
        help="Atlas ID",
    ),
) -> None:
    """Show detailed information about an Atlas."""
    config = get_config()
    runtime_url = config.runtime.url

    try:
        response = httpx.get(
            f"{runtime_url}/v1/atlases/{atlas_id}",
            timeout=config.runtime.timeout_ms / 1000,
        )

        if response.status_code == 404:
            console.print(f"[red]Atlas not found: {atlas_id}[/red]")
            raise typer.Exit(1)

        response.raise_for_status()
        data = response.json()

        console.print()
        console.print(f"[bold]{data['name']}[/bold]")
        console.print(f"ID: {data['id']} v{data['version']}")
        console.print()

        if data["description"]:
            console.print(data["description"])
            console.print()

        console.print("[bold]Capabilities:[/bold]")
        for cap in data["capabilities"]:
            console.print(f"  - {cap}")
        console.print()

        console.print("[bold]Resources:[/bold]")
        console.print(f"  Context Packs: {data['context_packs']}")
        console.print(f"  Policies:      {data['policies']}")
        console.print()

        console.print("[bold]Adapters:[/bold]")
        for adapter in data["adapters"]:
            console.print(f"  - {adapter}")
        console.print()

        console.print("[bold]Certification:[/bold]")
        cert = data["certification"]
        carp_status = "[green]Yes[/green]" if cert.get("carp_compliant") else "[red]No[/red]"
        trace_status = "[green]Yes[/green]" if cert.get("trace_compliant") else "[red]No[/red]"
        console.print(f"  CARP Compliant:  {carp_status}")
        console.print(f"  TRACE Compliant: {trace_status}")

    except httpx.ConnectError:
        console.print(f"[red]Cannot connect to runtime at {runtime_url}[/red]")
        raise typer.Exit(1)


@app.command("unload")
def unload_atlas(
    atlas_id: str = typer.Argument(
        ...,
        help="Atlas ID to unload",
    ),
) -> None:
    """Unload an Atlas from the runtime."""
    config = get_config()
    runtime_url = config.runtime.url

    try:
        response = httpx.delete(
            f"{runtime_url}/v1/atlases/{atlas_id}",
            timeout=config.runtime.timeout_ms / 1000,
        )

        if response.status_code == 404:
            console.print(f"[red]Atlas not found: {atlas_id}[/red]")
            raise typer.Exit(1)

        response.raise_for_status()
        console.print(f"[green]Atlas unloaded: {atlas_id}[/green]")

    except httpx.ConnectError:
        console.print(f"[red]Cannot connect to runtime at {runtime_url}[/red]")
        raise typer.Exit(1)


@app.command("emit")
def emit_atlas(
    atlas_id: str = typer.Argument(
        ...,
        help="Atlas ID",
    ),
    platform: str = typer.Option(
        "openai",
        "--platform",
        "-p",
        help="Target platform (openai, anthropic, google_adk, mcp)",
    ),
    output: Path = typer.Option(
        None,
        "--output",
        "-o",
        help="Output file path",
    ),
) -> None:
    """Emit Atlas in platform-specific format.

    Generates tool definitions and context for the specified platform.
    """
    config = get_config()
    runtime_url = config.runtime.url

    valid_platforms = ["openai", "anthropic", "google_adk", "mcp"]
    if platform not in valid_platforms:
        console.print(f"[red]Invalid platform: {platform}[/red]")
        console.print(f"Valid platforms: {', '.join(valid_platforms)}")
        raise typer.Exit(1)

    try:
        response = httpx.get(
            f"{runtime_url}/v1/atlases/{atlas_id}/emit/{platform}",
            timeout=config.runtime.timeout_ms / 1000,
        )

        if response.status_code == 404:
            console.print(f"[red]Atlas not found: {atlas_id}[/red]")
            raise typer.Exit(1)

        response.raise_for_status()
        data = response.json()

        if output:
            import json
            with open(output, "w") as f:
                json.dump(data["output"], f, indent=2)
            console.print(f"[green]Output written to: {output}[/green]")
        else:
            console.print(f"[bold]Platform: {platform}[/bold]")
            console.print()
            console.print(JSON.from_data(data["output"]))

    except httpx.ConnectError:
        console.print(f"[red]Cannot connect to runtime at {runtime_url}[/red]")
        raise typer.Exit(1)


@app.command("context")
def show_context(
    atlas_id: str = typer.Argument(
        ...,
        help="Atlas ID",
    ),
) -> None:
    """Show context blocks from an Atlas."""
    config = get_config()
    runtime_url = config.runtime.url

    try:
        response = httpx.get(
            f"{runtime_url}/v1/atlases/{atlas_id}/context",
            timeout=config.runtime.timeout_ms / 1000,
        )

        if response.status_code == 404:
            console.print(f"[red]Atlas not found: {atlas_id}[/red]")
            raise typer.Exit(1)

        response.raise_for_status()
        data = response.json()

        console.print(f"[bold]Context Blocks ({data['count']})[/bold]")
        console.print()

        for block in data["blocks"]:
            console.print(f"[cyan]{block['block_id']}[/cyan]")
            console.print(f"  Purpose: {block['purpose']}")
            console.print(f"  Type:    {block['content_type']}")
            console.print(f"  TTL:     {block['ttl_seconds']}s")
            console.print()

    except httpx.ConnectError:
        console.print(f"[red]Cannot connect to runtime at {runtime_url}[/red]")
        raise typer.Exit(1)


@app.command("actions")
def show_actions(
    atlas_id: str = typer.Argument(
        ...,
        help="Atlas ID",
    ),
) -> None:
    """Show allowed actions from an Atlas."""
    config = get_config()
    runtime_url = config.runtime.url

    try:
        response = httpx.get(
            f"{runtime_url}/v1/atlases/{atlas_id}/actions",
            timeout=config.runtime.timeout_ms / 1000,
        )

        if response.status_code == 404:
            console.print(f"[red]Atlas not found: {atlas_id}[/red]")
            raise typer.Exit(1)

        response.raise_for_status()
        data = response.json()

        console.print(f"[bold]Actions ({data['count']})[/bold]")
        console.print()

        for action in data["actions"]:
            console.print(f"[cyan]{action['action_id']}[/cyan]")
            console.print(f"  Kind:    {action['kind']}")
            console.print(f"  Adapter: {action['adapter']}")
            if action.get("description"):
                console.print(f"  Desc:    {action['description']}")
            console.print()

    except httpx.ConnectError:
        console.print(f"[red]Cannot connect to runtime at {runtime_url}[/red]")
        raise typer.Exit(1)
