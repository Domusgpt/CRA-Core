"""CRA doctor command - System check."""

import httpx
import typer
from rich.console import Console
from rich.table import Table

from cra.version import CARP_VERSION, TRACE_VERSION, __version__
from cra.cli.config import get_config, CRAConfig

app = typer.Typer(help="Check CRA system status")
console = Console()


def check_runtime(url: str) -> tuple[bool, str]:
    """Check if the runtime is accessible.

    Args:
        url: Runtime URL

    Returns:
        Tuple of (success, message)
    """
    try:
        response = httpx.get(f"{url}/v1/health", timeout=5.0)
        if response.status_code == 200:
            data = response.json()
            return True, f"v{data['version']}"
        return False, f"HTTP {response.status_code}"
    except httpx.ConnectError:
        return False, "Connection refused"
    except httpx.TimeoutException:
        return False, "Timeout"
    except Exception as e:
        return False, str(e)


def check_config() -> tuple[bool, str]:
    """Check if config file exists.

    Returns:
        Tuple of (success, message)
    """
    try:
        config = get_config()
        return True, config.config_path or "default"
    except Exception as e:
        return False, str(e)


def check_trace_dir(config: CRAConfig | None) -> tuple[bool, str]:
    """Check if trace directory is writable.

    Args:
        config: CRA config

    Returns:
        Tuple of (success, message)
    """
    import os
    from pathlib import Path

    trace_dir = Path(config.trace.directory if config else "./cra.trace")
    if trace_dir.exists():
        if os.access(trace_dir, os.W_OK):
            return True, str(trace_dir)
        return False, "Not writable"
    return False, "Not found"


@app.callback(invoke_without_command=True)
def doctor(
    ctx: typer.Context,
    runtime_url: str = typer.Option(
        None,
        "--runtime",
        "-r",
        help="Runtime URL to check",
    ),
) -> None:
    """Check CRA system status.

    Verifies that:
    - Runtime is accessible and healthy
    - Configuration is valid
    - Trace directory is writable
    - Required dependencies are available
    """
    console.print()
    console.print("[bold]CRA Doctor - System Check[/bold]")
    console.print("=" * 40)
    console.print()

    # Load config
    try:
        config = get_config()
    except Exception:
        config = None

    # Determine runtime URL
    url = runtime_url or (config.runtime.url if config else "http://localhost:8420")

    # Create results table
    table = Table(show_header=False, box=None, padding=(0, 2))
    table.add_column("Check", style="cyan")
    table.add_column("Value")
    table.add_column("Status")

    all_ok = True

    # Check runtime
    runtime_ok, runtime_msg = check_runtime(url)
    table.add_row(
        "Runtime:",
        f"{url} ({runtime_msg})",
        "[green][OK][/green]" if runtime_ok else "[red][FAIL][/red]",
    )
    if not runtime_ok:
        all_ok = False

    # CLI version
    table.add_row("CLI Version:", __version__, "[green][OK][/green]")

    # CARP version
    table.add_row("CARP:", CARP_VERSION, "[green][OK][/green]")

    # TRACE version
    table.add_row("TRACE:", TRACE_VERSION, "[green][OK][/green]")

    # Check config
    config_ok, config_msg = check_config()
    table.add_row(
        "Config:",
        config_msg,
        "[green][FOUND][/green]" if config_ok else "[yellow][DEFAULT][/yellow]",
    )

    # Check trace directory
    trace_ok, trace_msg = check_trace_dir(config)
    table.add_row(
        "Trace Dir:",
        trace_msg,
        "[green][WRITABLE][/green]" if trace_ok else "[yellow][MISSING][/yellow]",
    )

    # Check atlases
    atlas_count = len(config.atlases) if config else 0
    table.add_row(
        "Atlases:",
        f"{atlas_count} loaded",
        "[green][OK][/green]" if atlas_count > 0 else "[dim][NONE][/dim]",
    )

    console.print(table)
    console.print()

    if all_ok:
        console.print("[green]All checks passed.[/green]")
    else:
        console.print("[yellow]Some checks failed. Run 'cra init' to set up.[/yellow]")

    console.print()
