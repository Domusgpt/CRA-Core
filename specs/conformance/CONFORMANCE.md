# CRA Conformance Test Suite

This directory contains conformance tests that any CRA implementation MUST pass.

## Test Categories

### 1. Schema Validation

All JSON structures MUST validate against the schemas in `../schemas/`:
- `carp-request.schema.json` - CARP request format
- `carp-resolution.schema.json` - CARP resolution format
- `trace-event.schema.json` - TRACE event format
- `atlas-manifest.schema.json` - Atlas manifest format

### 2. Hash Chain Verification

The TRACE hash chain MUST be computed as:

```
event_hash = SHA256(
  trace_version ||
  event_id ||
  trace_id ||
  span_id ||
  parent_span_id ||
  session_id ||
  sequence ||
  timestamp ||
  event_type ||
  canonical_json(payload) ||
  previous_event_hash
)
```

Where:
- `||` means concatenation
- `canonical_json()` produces deterministic JSON with sorted keys
- Genesis event uses `previous_event_hash = "0000...0000"` (64 zeros)

### 3. Policy Evaluation Order

Policies MUST be evaluated in this order:
1. **Deny** - Immediate rejection
2. **Requires Approval** - Mark for human approval
3. **Rate Limit** - Check and update counters
4. **Allow** - Explicit allowance

If no policy matches, the action is allowed by default.

### 4. Golden Traces

The `golden/` directory contains reference test cases:

```
golden/
├── simple-resolve/
│   ├── atlas.json           # Input atlas
│   ├── request.json         # Input CARP request
│   ├── expected-resolution.json  # Expected output
│   └── expected-trace.jsonl      # Expected TRACE events
└── ...
```

For each golden test:
1. Load the atlas
2. Submit the request
3. Compare resolution against expected (ignoring timestamps/UUIDs)
4. Compare trace structure against expected (ignoring timestamps/UUIDs)

## Running Conformance Tests

### Rust

```bash
cargo test --features conformance
```

### Python

```bash
python -m pytest tests/conformance/
```

### Node.js

```bash
npm run test:conformance
```

## Conformance Levels

### Level 1: Core (Required)

- [ ] Schema validation passes
- [ ] Hash chain computation is correct
- [ ] Policy evaluation order is correct
- [ ] Basic resolve/execute flow works

### Level 2: Complete (Recommended)

- [ ] All golden traces match
- [ ] Replay produces identical results
- [ ] Chain verification detects tampering
- [ ] Rate limiting works correctly

### Level 3: Extended (Optional)

- [ ] Multi-atlas resolution
- [ ] Capability-based access control
- [ ] Context block injection
- [ ] Approval workflow support
