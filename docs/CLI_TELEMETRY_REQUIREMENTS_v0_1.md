# CLI Telemetry Requirements v0.1

- CLI is asynchronous by default and streams TRACE events as JSONL
- Filtering by trace_id, severity, and event_type must be available
- Tail/follow mode must not drop events
- CLI scaffolds projects by creating `agents.md`, `config/`, `lock/`, and `traces/`
- Runtime is authoritative; CLI only renders events and does not rewrite telemetry
- Support exporting golden traces for regression tests via `cra export-trace`
- Provide `cra list-traces` to show available sessions and `cra replay` to validate TRACE streams against schema
- Provide `cra invoke-action` for adapter calls with approval, rate limit, and license telemetry
- Provide `cra license-register` to persist Atlas license keys under `config/licenses.json`
- Provide `cra identity-register` to persist identity tokens/scopes under `config/identities.json` and optionally enforce token requirement
- Action governance events must surface in CLI output: `trace.action.policy.denied`, `trace.action.pending_approval`, and `trace.action.rate_limited`
