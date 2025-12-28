from __future__ import annotations

import json
import shutil
from pathlib import Path
from typing import List, Optional

import typer

from .auth import AuthManager
from .context import ContextManager
from .runtime import CRARuntime
from .trace import TraceEmitter, TraceStore
from .license import LicenseManager
from .models import Session, TraceIds
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


def _split_scopes(raw: str) -> List[str]:
    scopes = [s.strip() for s in raw.split(",") if s.strip()]
    return sorted(set(scopes))


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
    principal_id: str = typer.Option("cli", help="Principal identifier for the session"),
    principal_type: str = typer.Option("user", help="Principal type: user|service|agent"),
    scopes: str = typer.Option("", help="Comma-delimited scopes to request"),
    token: Optional[str] = typer.Option(None, help="Identity token for authz"),
    refresh_context: bool = typer.Option(
        False, help="Force TTL refresh for context blocks before resolve"
    ),
) -> None:
    """Send a CARP resolve request and stream TRACE to stdout."""
    ensure_scaffold(Path("."))
    runtime = CRARuntime(atlas_path=atlas)
    session = Session(
        principal_id=principal_id, principal_type=principal_type, scopes=_split_scopes(scopes)
    )
    response = runtime.resolve(
        goal=goal,
        risk_tier=risk_tier,
        session=session,
        token=token,
        refresh_context=refresh_context,
    )
    typer.echo(json.dumps(response, indent=2))


@app.command("invoke-action")
def invoke_action(
    action_id: str = typer.Argument(..., help="Action id to invoke (adapter.action)"),
    payload: str = typer.Option("{}", help="JSON payload for the action schema"),
    atlas: Path = typer.Option(Path("atlas/reference"), help="Path to Atlas root"),
    approve: bool = typer.Option(False, "--approve", help="Auto-approve high-risk actions"),
    principal_id: str = typer.Option("cli", help="Principal identifier for the session"),
    principal_type: str = typer.Option("user", help="Principal type: user|service|agent"),
    scopes: str = typer.Option("", help="Comma-delimited scopes"),
    token: Optional[str] = typer.Option(None, help="Identity token for authz"),
) -> None:
    """Simulate an adapter invocation with approvals, licensing, and rate limits."""
    ensure_scaffold(Path("."))
    runtime = CRARuntime(atlas_path=atlas)
    try:
        payload_dict = json.loads(payload)
    except json.JSONDecodeError as exc:  # noqa: TRY003
        typer.echo(f"Invalid payload JSON: {exc}")
        raise typer.Exit(code=1)

    session = Session(
        principal_id=principal_id, principal_type=principal_type, scopes=_split_scopes(scopes)
    )
    result = runtime.invoke_action(
        action_id=action_id,
        payload=payload_dict,
        auto_approve=approve,
        session=session,
        token=token,
    )
    typer.echo(json.dumps(result, indent=2))


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


@app.command("license-register")
def license_register(
    atlas_id: str = typer.Argument(..., help="Atlas identifier"),
    key: str = typer.Argument(..., help="License key"),
    model: str = typer.Option("subscription", help="License model"),
) -> None:
    """Register a license key in config/licenses.json."""
    manager = LicenseManager()
    record = manager.register(atlas_id=atlas_id, key=key, model=model)
    typer.echo(json.dumps({"atlas_id": atlas_id, "record": record}, indent=2))


@app.command("identity-register")
def identity_register(
    token: str = typer.Argument(..., help="Bearer token for the identity"),
    principal_id: str = typer.Argument(..., help="Principal identifier"),
    principal_type: str = typer.Option("user", help="Principal type: user|service|agent"),
    scopes: str = typer.Option("", help="Comma-delimited scopes"),
    expires_at: Optional[str] = typer.Option(None, help="Expiry in ISO8601 (UTC)"),
    require_token: bool = typer.Option(False, help="Enable token requirement flag"),
) -> None:
    """Register an identity token in config/identities.json."""
    manager = AuthManager()
    record = manager.register(
        token=token,
        principal_id=principal_id,
        principal_type=principal_type,
        scopes=_split_scopes(scopes),
        expires_at=expires_at,
    )
    if require_token:
        manager.toggle_require_token(True)
    typer.echo(json.dumps({"token": token, "record": record, "require_token": manager.require_token}, indent=2))


@app.command("context-status")
def context_status() -> None:
    """Inspect current context registry TTL state."""

    ensure_scaffold(Path("."))
    manager = ContextManager()
    typer.echo(json.dumps(manager.status(), indent=2))


@app.command("context-refresh")
def context_refresh(
    block_id: Optional[str] = typer.Option(None, help="Specific block id to refresh; defaults to all"),
) -> None:
    """Clear context registry entries to force reissuance on next resolve."""

    ensure_scaffold(Path("."))
    manager = ContextManager()
    state = manager.refresh(block_id=block_id)
    typer.echo(json.dumps({"refreshed": block_id or "all", "state": state}, indent=2))


@app.command("list-traces")
def list_traces() -> None:
    """List available TRACE files with metadata."""
    store = TraceStore()
    entries = store.list_traces()
    if not entries:
        typer.echo("No traces available.")
        raise typer.Exit(code=1)
    typer.echo(json.dumps(entries, indent=2))


@app.command("export-trace")
def export_trace(
    trace: Optional[str] = typer.Option(None, "--trace-id", "-t", help="Trace id or 'latest'"),
    output: Path = typer.Option(Path("traces/export/trace-manifest.json"), help="Where to write manifest"),
) -> None:
    """Export a TRACE manifest (hash + metadata) for golden-trace workflows."""
    trace_id = trace
    if trace_id in {None, "latest"}:
        trace_id = TraceEmitter.latest_trace_id()
    if not trace_id:
        typer.echo("No trace files found.")
        raise typer.Exit(code=1)
    store = TraceStore()
    manifest = store.export_manifest(trace_id, output)
    typer.echo(json.dumps(manifest, indent=2))


@app.command()
def replay(
    trace_file: Path = typer.Argument(..., help="Path to TRACE JSONL file"),
    fail_fast: bool = typer.Option(False, help="Stop on first validation error"),
) -> None:
    """Validate a TRACE stream for replay/regression purposes."""
    store = TraceStore(trace_file.parent)
    count, errors = store.replay(trace_file=trace_file, fail_fast=fail_fast)
    summary = {"events": count, "errors": errors, "status": "pass" if not errors else "fail"}
    typer.echo(json.dumps(summary, indent=2))
    if errors:
        raise typer.Exit(code=1)


if __name__ == "__main__":
    app()
