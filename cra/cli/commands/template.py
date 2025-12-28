"""CRA template command - Generate agent templates."""

from pathlib import Path

import typer
from rich.console import Console
from rich.panel import Panel
from rich.table import Table

from cra.cli.config import get_config
from cra.core.atlas import AtlasLoader
from cra.templates import get_template_generator

app = typer.Typer(help="Generate agent templates")
console = Console()


SUPPORTED_FRAMEWORKS = ["openai_gpt", "langchain", "crewai"]


@app.command("list")
def list_frameworks() -> None:
    """List supported frameworks for template generation."""
    table = Table(title="Supported Frameworks")
    table.add_column("Framework", style="cyan")
    table.add_column("Description")
    table.add_column("Version")

    frameworks = [
        ("openai_gpt", "OpenAI GPT Actions (Custom GPTs)", "2024-01"),
        ("langchain", "LangChain/LangGraph agents", "0.1.0"),
        ("crewai", "CrewAI multi-agent crews", "0.28.0"),
    ]

    for name, desc, version in frameworks:
        table.add_row(name, desc, version)

    console.print(table)


@app.command("generate")
def generate_template(
    atlas_path: Path = typer.Argument(
        ...,
        help="Path to Atlas directory",
    ),
    framework: str = typer.Option(
        "langchain",
        "--framework",
        "-f",
        help="Target framework (openai_gpt, langchain, crewai)",
    ),
    output: Path = typer.Option(
        None,
        "--output",
        "-o",
        help="Output directory (default: ./generated/<framework>)",
    ),
    use_langgraph: bool = typer.Option(
        True,
        "--langgraph/--no-langgraph",
        help="Use LangGraph (for langchain framework)",
    ),
) -> None:
    """Generate agent template from an Atlas.

    Creates framework-specific code and configuration
    for building CRA-governed agents.
    """
    if framework not in SUPPORTED_FRAMEWORKS:
        console.print(f"[red]Unknown framework: {framework}[/red]")
        console.print(f"Supported: {', '.join(SUPPORTED_FRAMEWORKS)}")
        raise typer.Exit(1)

    if not atlas_path.exists():
        console.print(f"[red]Atlas path not found: {atlas_path}[/red]")
        raise typer.Exit(1)

    # Load the Atlas
    try:
        loader = AtlasLoader()
        atlas = loader.load(atlas_path)
    except Exception as e:
        console.print(f"[red]Failed to load Atlas: {e}[/red]")
        raise typer.Exit(1)

    # Get the template generator
    try:
        generator = get_template_generator(framework)
    except ValueError as e:
        console.print(f"[red]{e}[/red]")
        raise typer.Exit(1)

    # Configure generation
    config = {}
    if framework == "langchain":
        config["use_langgraph"] = use_langgraph

    # Generate template
    console.print(f"Generating [cyan]{framework}[/cyan] template for [bold]{atlas.manifest.name}[/bold]...")

    try:
        template = generator.generate(atlas, config=config)
    except Exception as e:
        console.print(f"[red]Generation failed: {e}[/red]")
        raise typer.Exit(1)

    # Determine output directory
    if output is None:
        output = Path("generated") / framework

    # Write files
    written = template.write_to_directory(output)

    console.print()
    console.print(Panel(
        f"[green]Generated {len(written)} files[/green]",
        title="Success",
    ))

    console.print()
    console.print("[bold]Generated Files:[/bold]")
    for gen_file in template.files:
        icon = "📄" if not gen_file.executable else "🔧"
        console.print(f"  {icon} {output / gen_file.path}")
        if gen_file.description:
            console.print(f"     [dim]{gen_file.description}[/dim]")

    console.print()
    console.print("[bold]Dependencies:[/bold]")
    for dep in template.dependencies[:5]:
        console.print(f"  - {dep}")
    if len(template.dependencies) > 5:
        console.print(f"  ... and {len(template.dependencies) - 5} more")

    console.print()
    console.print("[bold]Next Steps:[/bold]")
    console.print(f"  cd {output}")
    console.print("  pip install -r requirements.txt")
    console.print("  python main.py")


@app.command("info")
def template_info(
    framework: str = typer.Argument(
        ...,
        help="Framework name",
    ),
) -> None:
    """Show detailed information about a framework template."""
    if framework not in SUPPORTED_FRAMEWORKS:
        console.print(f"[red]Unknown framework: {framework}[/red]")
        console.print(f"Supported: {', '.join(SUPPORTED_FRAMEWORKS)}")
        raise typer.Exit(1)

    try:
        generator = get_template_generator(framework)
    except ValueError as e:
        console.print(f"[red]{e}[/red]")
        raise typer.Exit(1)

    console.print()
    console.print(f"[bold]{generator.framework_name}[/bold]")
    console.print(f"Minimum version: {generator.framework_version}")
    console.print()

    if framework == "openai_gpt":
        console.print("""[bold]OpenAI GPT Actions[/bold]

Generates configuration for custom GPTs with CRA-governed actions.

Files generated:
- openapi.json - OpenAPI spec for GPT Actions
- instructions.md - GPT system instructions
- privacy-policy.md - Privacy policy template
- server.py - Bridge server (FastAPI)

Use case:
- Building custom GPTs that need governed tool access
- Enterprise GPT deployments with audit requirements
- GPTs that interact with internal systems
""")

    elif framework == "langchain":
        console.print("""[bold]LangChain/LangGraph[/bold]

Generates LangChain tools and agents backed by CRA governance.

Files generated:
- cra_tools.py - LangChain tools with CRA integration
- cra_agent.py - Agent implementation (LangGraph or classic)
- main.py - Interactive example
- requirements.txt - Dependencies

Options:
- --langgraph: Use LangGraph (default: True)
- --no-langgraph: Use classic LangChain agent

Use case:
- Building governed RAG applications
- Enterprise chatbots with audit trails
- Complex agent workflows with compliance needs
""")

    elif framework == "crewai":
        console.print("""[bold]CrewAI[/bold]

Generates multi-agent crews with CRA governance.

Files generated:
- cra_tools.py - CrewAI tools with CRA integration
- agents.py - Agent definitions (Researcher, Executor, Reviewer)
- tasks.py - Task templates
- crew.py - Crew orchestration
- main.py - Example usage
- requirements.txt - Dependencies

Use case:
- Multi-agent workflows with governance
- Complex automation with audit requirements
- Collaborative AI systems with compliance
""")
