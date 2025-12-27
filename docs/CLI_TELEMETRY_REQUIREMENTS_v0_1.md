# CLI Telemetry Requirements v0.1

- CLI is asynchronous by default and streams TRACE events as JSONL
- Filtering by trace_id, severity, and event_type must be available
- Tail/follow mode must not drop events
- CLI scaffolds projects by creating `agents.md`, `config/`, `lock/`, and `traces/`
- Runtime is authoritative; CLI only renders events and does not rewrite telemetry
- Support exporting golden traces for regression tests
