# TRACE/1.0 — Telemetry & Replay Artifact Contract (Draft v0.1)

## Principle
**If it wasn’t emitted by the runtime, it didn’t happen.**

## Event Envelope
```json
{
  "trace_version": "1.0",
  "event_type": "string",
  "time": "RFC3339",
  "trace": {"trace_id":"uuid","span_id":"uuid","parent_span_id":"uuid|null"},
  "session_id": "uuid",
  "atlas": {"id":"string","version":"semver"},
  "actor": {"type":"runtime|agent|user|tool","id":"string"},
  "severity": "debug|info|warn|error",
  "payload": {},
  "artifacts": [{"name":"string","uri":"string","sha256":"string","content_type":"string"}]
}
```

## Mandatory Event Types
- `trace.session.started|ended`
- `trace.carp.resolve.requested|returned|policy.denied`
- `trace.action.granted|invoked|completed|failed`
- `trace.artifact.created|updated|redacted`
- `trace.runtime.error|trace.adapter.error|trace.validation.error`

## Replay
- Export a replay manifest referencing trace_id plus required artifacts
- Allow declared nondeterminism via redaction rules
- Golden traces become baselines for regression tests

## CLI Streaming Requirements
- JSONL streaming
- Filterable (trace_id, severity, event_type)
- Tail/follow mode
