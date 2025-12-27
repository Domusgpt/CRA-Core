from __future__ import annotations

import json
import shutil
from pathlib import Path
from typing import Optional

import typer

from .runtime import CRARuntime
from .trace import TraceEmitter
from .models import TraceIds
from .validators import SchemaValidator, ValidationError

app = typer.Typer(help="CRA telemetry-first CLI")


def ensure_scaffold(project_root: Path) -> None:
    (project_root / "config").mkdir(parents=True, exist_ok=True)
    (project_root / "lock").mkdir(parents=True, exist_ok=True)
    (project_root / "traces").mkdir(parents=True, exist_ok=True)
    target_agents = project_root / "agents.md"
    if not target_agents.exists():
        template = Path(__file__).parent.parent / "templates" / "agents.md"
        if template.exists():
            shutil.copy(template, target_agents)
    target_prompt = project_root / "CRA_STARTER_PROMPT.md"
    if not target_prompt.exists():
        template = Path(__file__).parent.parent / "templates" / "CRA_STARTER_PROMPT.md"
        if template.exists():
            shutil.copy(template, target_prompt)


@app.command()
def init(path: Path = typer.Argument(Path("."), help="Project root")) -> None:
    """Initialize CRA project scaffolding."""
    ensure_scaffold(path)
    typer.echo(f"Initialized CRA project at {path.resolve()}")


@app.command()
def resolve(
    goal: str = typer.Argument(..., help="Goal to resolve via CARP"),
    atlas: Path = typer.Option(Path("atlas/reference"), help="Path to Atlas root"),
    risk_tier: str = typer.Option("low", help="Risk tier: low|medium|high"),
) -> None:
    """Send a CARP resolve request and stream TRACE to stdout."""
    ensure_scaffold(Path("."))
    runtime = CRARuntime(atlas_path=atlas)
    response = runtime.resolve(goal=goal, risk_tier=risk_tier)
    typer.echo(json.dumps(response, indent=2))


@app.command()
def tail(
    trace: Optional[str] = typer.Option(None, "--trace-id", "-t", help="Trace id to follow or 'latest'"),
    follow: bool = typer.Option(False, "--follow", "-f", is_flag=True, help="Follow new events"),
    event_type: Optional[str] = typer.Option(None, help="Filter by event type"),
    severity: Optional[str] = typer.Option(None, help="Filter by severity"),
) -> None:
    """Tail TRACE events from a trace file."""
    trace_id = trace
    if trace_id in {None, "latest"}:
        trace_id = TraceEmitter.latest_trace_id()
    if not trace_id:
        typer.echo("No trace files found.")
        raise typer.Exit(code=1)

    emitter = TraceEmitter(trace_ids=TraceIds(trace_id=trace_id))
    emitter.trace_file = Path("traces") / f"{trace_id}.jsonl"
    if not emitter.trace_file.exists():
        typer.echo(f"Trace file for id {trace_id} not found")
        raise typer.Exit(code=1)

    for event in emitter.tail(follow=follow, event_type=event_type, severity=severity):
        typer.echo(json.dumps(event))


@app.command()
def validate(atlas: Path = typer.Option(Path("atlas/reference"), help="Path to Atlas root")) -> None:
    """Validate Atlas manifest against schema."""
    validator = SchemaValidator()
    manifest_path = atlas / "atlas.json"
    if not manifest_path.exists():
        typer.echo(f"Atlas manifest not found at {manifest_path}")
        raise typer.Exit(code=1)
    manifest = json.loads(manifest_path.read_text())
    try:
        validator.validate("atlas", manifest)
    except ValidationError as exc:
        typer.echo(f"Atlas validation failed: {exc}")
        raise typer.Exit(code=1)
    typer.echo(f"Atlas manifest at {manifest_path} is valid.")


if __name__ == "__main__":
    app()
