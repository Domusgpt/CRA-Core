"""CRA CLI main application."""

import typer
from rich.console import Console

from cra.version import __version__
from cra.cli.commands import doctor, init, resolve, trace

# Create the main CLI app
app = typer.Typer(
    name="cra",
    help="CRA - Context Registry Agents CLI",
    add_completion=False,
    no_args_is_help=True,
)

# Add subcommands
app.add_typer(doctor.app, name="doctor")
app.add_typer(init.app, name="init")
app.add_typer(resolve.app, name="resolve")
app.add_typer(trace.app, name="trace")

console = Console()


@app.callback()
def main(
    version: bool = typer.Option(
        False,
        "--version",
        "-v",
        help="Show version and exit",
        is_eager=True,
    ),
) -> None:
    """CRA - Context Registry Agents.

    A governed context layer that makes AI agents use tools,
    systems, and proprietary knowledge correctly.

    Run 'cra doctor' to check your setup.
    Run 'cra init' to initialize a new project.
    """
    if version:
        console.print(f"cra version {__version__}")
        raise typer.Exit()


if __name__ == "__main__":
    app()
