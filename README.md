# CRA-Core

Context Registry Agents (CRA) provide an authority layer that resolves governed context and permitted actions for agentic systems. This repository includes protocol drafts for CARP and TRACE, an in-memory reference runtime, a telemetry-first CLI, and a reference Atlas.

## Contents
- `docs/` — CARP/TRACE specs, runtime and CLI requirements, architecture plan, roadmap
- `src/cra/` — runtime, models, telemetry, and CLI
- `atlas/reference/` — minimal Atlas demonstrating context blocks and adapter metadata
- `templates/` — starter agent contract and CRA prompt
- `config/`, `lock/`, `traces/` — created/maintained by the CLI

## Quickstart
1. Install dependencies (and the CLI entry point)
   ```bash
   pip install -r requirements.txt
   pip install -e .
   ```
2. Initialize a project (creates `agents.md`, `config/`, `lock/`, `traces/`)
   ```bash
   python -m cra.cli init
   ```
3. Resolve a goal against the reference Atlas and stream TRACE
   ```bash
   python -m cra.cli resolve "Summarize CRA architecture" --atlas atlas/reference
   python -m cra.cli tail --trace latest
   ```
4. Register an identity token (optional but required when token enforcement is enabled) and invoke an allowed adapter action with telemetry, approvals, rate limits, and scope checks
   ```bash
   python -m cra.cli identity-register demo-token cli-user --scopes repo.read,inventory.read
   python -m cra.cli invoke-action openai.tools.fetch_repo_status --payload '{"path":"."}' --approve --token demo-token
   ```

5. Export and replay TRACE for golden-trace regression checks
   ```bash
   python -m cra.cli list-traces
   python -m cra.cli export-trace --trace-id latest --output traces/export/trace-manifest.json
   python -m cra.cli replay traces/$(cat traces/latest).jsonl
   ```

## Protocols
- CARP/1.0 — Context & Action Resolution Protocol (`docs/CARP_1_0.md`)
- TRACE/1.0 — Telemetry & Replay Artifact Contract (`docs/TRACE_1_0.md`)

## Governance
- Context blocks are TTL-bounded, persisted in `lock/context_registry.json`, and redacted once expired; TRACE emits `trace.artifact.created|updated|redacted` with TTL metadata
- Actions are constrained by adapters, scopes, approvals, rate limits (with window hints), Atlas licensing hooks, and optional token-gated identities with scope validation
- TRACE events are append-only; if telemetry does not show it, it did not happen
- Governance telemetry includes policy denials, pending approvals, and rate-limit events alongside granted/invoked/completed/failure events
