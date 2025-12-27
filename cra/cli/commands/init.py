"""CRA init command - Initialize a new project."""

from pathlib import Path

import typer
from rich.console import Console

from cra.version import __version__
from cra.cli.config import (
    CRAConfig,
    generate_agents_md,
    save_config,
)

app = typer.Typer(help="Initialize a CRA project")
console = Console()


@app.callback(invoke_without_command=True)
def init(
    ctx: typer.Context,
    runtime_url: str = typer.Option(
        "http://localhost:8420",
        "--runtime",
        "-r",
        help="Runtime URL",
    ),
    force: bool = typer.Option(
        False,
        "--force",
        "-f",
        help="Overwrite existing files",
    ),
) -> None:
    """Initialize a new CRA project.

    Creates:
    - agents.md: Agent behavior contract
    - cra.config.json: Runtime configuration
    - cra.trace/: Local trace storage directory
    - cra.atlases.lock: Atlas version lock file
    """
    cwd = Path.cwd()

    console.print()
    console.print("[bold]Initializing CRA project...[/bold]")
    console.print()

    # Create configuration
    config = CRAConfig()
    config.runtime.url = runtime_url

    files_created = []

    # Create agents.md
    agents_path = cwd / "agents.md"
    if agents_path.exists() and not force:
        console.print(f"[yellow]Skipped:[/yellow] agents.md (exists)")
    else:
        agents_content = generate_agents_md(config, __version__)
        agents_path.write_text(agents_content)
        files_created.append("agents.md")
        console.print(f"[green]Created:[/green] agents.md")

    # Create cra.config.json
    config_path = cwd / "cra.config.json"
    if config_path.exists() and not force:
        console.print(f"[yellow]Skipped:[/yellow] cra.config.json (exists)")
    else:
        save_config(config, config_path)
        files_created.append("cra.config.json")
        console.print(f"[green]Created:[/green] cra.config.json")

    # Create cra.trace directory
    trace_dir = cwd / "cra.trace"
    if trace_dir.exists():
        console.print(f"[yellow]Skipped:[/yellow] cra.trace/ (exists)")
    else:
        trace_dir.mkdir(parents=True, exist_ok=True)
        # Create .gitkeep
        (trace_dir / ".gitkeep").touch()
        files_created.append("cra.trace/")
        console.print(f"[green]Created:[/green] cra.trace/")

    # Create cra.atlases.lock
    lock_path = cwd / "cra.atlases.lock"
    if lock_path.exists() and not force:
        console.print(f"[yellow]Skipped:[/yellow] cra.atlases.lock (exists)")
    else:
        lock_content = """{
  "locked_atlases": [],
  "locked_at": null
}
"""
        lock_path.write_text(lock_content)
        files_created.append("cra.atlases.lock")
        console.print(f"[green]Created:[/green] cra.atlases.lock")

    console.print()

    if files_created:
        console.print(f"[green]Project initialized with {len(files_created)} files.[/green]")
    else:
        console.print("[yellow]No files created (all exist). Use --force to overwrite.[/yellow]")

    console.print()
    console.print("[dim]Run 'cra doctor' to verify your setup.[/dim]")
    console.print()
