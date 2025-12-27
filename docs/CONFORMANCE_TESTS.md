# CARP/TRACE Conformance & Testing Approach

## Structural Validation
- Validate CARP envelopes contain session, atlas, payload, and trace blocks
- Ensure TRACE events include `trace_version`, `event_type`, timestamps, actor, severity, and payload
- Reject TRACE streams that skip mandatory event types for a session (`trace.session.started`/`ended`, resolve events)
- Enforce JSON Schema validation for CARP request/response, TRACE events, and Atlas manifests prior to runtime processing

## Golden Traces
- Capture TRACE JSONL for representative flows and store under `traces/fixtures/`
- On regression runs, replay TRACE to ensure event ordering and payload hashes match (allow listed redactions supported)

## Runtime Checks
- Enforce TTL on context blocks; expired blocks must be redacted in responses
- Verify allowed_actions align with Atlas adapter schemas
- Confirm denylist patterns are present for medium/high risk tiers

## CLI Checks
- `cra init` creates `agents.md`, `config/`, `lock/`, `traces/`
- `cra resolve` produces CARP response and emits TRACE JSONL
- `cra tail` filters by severity and event_type and supports follow mode
- `cra validate` confirms Atlas manifest schema compliance; validation failures must emit TRACE validation errors when encountered at runtime

## Marketplace/Certification (roadmap)
- Signed Atlas manifests with hash validation
- Policy tests for rate limits, approvals, and scope enforcement
- Usage metering against TRACE emission counts
