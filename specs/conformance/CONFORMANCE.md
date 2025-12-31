# CRA Conformance Test Specification

**Version:** 1.0.0
**Status:** Draft

---

## Overview

This document specifies the conformance requirements for CRA implementations. Any runtime claiming CRA compliance MUST pass all tests at its claimed conformance level.

## Conformance Levels

| Level | Description | Required Tests |
|-------|-------------|----------------|
| **CARP-Core** | Basic CARP resolution | Schema, Policy, Resolution |
| **TRACE-Core** | Basic trace emission | Events, Hash Chain, Replay |
| **Atlas-Core** | Atlas loading and validation | Manifest, Context, Actions |
| **CRA-Minimal** | All core levels | All above |
| **CRA-Full** | Complete implementation | All above + HTTP API |

---

## Test Categories

### 1. Schema Validation Tests

Implementations MUST correctly validate all protocol messages.

#### 1.1 CARP Request Validation

```yaml
test_id: schema.carp.request.valid
description: Valid CARP request passes validation
input:
  carp_version: "1.0"
  request_id: "01234567-89ab-cdef-0123-456789abcdef"
  timestamp: "2024-12-28T12:00:00.000Z"
  operation: "resolve"
  requester:
    agent_id: "test-agent"
    session_id: "01234567-89ab-cdef-0123-456789abcdef"
  task:
    goal: "Test goal"
expected: VALID
```

```yaml
test_id: schema.carp.request.invalid_version
description: Invalid CARP version fails validation
input:
  carp_version: "2.0"
  request_id: "01234567-89ab-cdef-0123-456789abcdef"
  timestamp: "2024-12-28T12:00:00.000Z"
  operation: "resolve"
  requester:
    agent_id: "test-agent"
    session_id: "01234567-89ab-cdef-0123-456789abcdef"
  task:
    goal: "Test goal"
expected: INVALID
error_path: "/carp_version"
```

```yaml
test_id: schema.carp.request.missing_required
description: Missing required field fails validation
input:
  carp_version: "1.0"
  request_id: "01234567-89ab-cdef-0123-456789abcdef"
  timestamp: "2024-12-28T12:00:00.000Z"
  operation: "resolve"
  requester:
    agent_id: "test-agent"
    session_id: "01234567-89ab-cdef-0123-456789abcdef"
  # task is missing
expected: INVALID
error_path: "/task"
```

```yaml
test_id: schema.carp.request.execute_requires_execution
description: Execute operation requires execution field
input:
  carp_version: "1.0"
  request_id: "01234567-89ab-cdef-0123-456789abcdef"
  timestamp: "2024-12-28T12:00:00.000Z"
  operation: "execute"
  requester:
    agent_id: "test-agent"
    session_id: "01234567-89ab-cdef-0123-456789abcdef"
  task:
    goal: "Execute action"
  # execution is missing
expected: INVALID
error_path: "/execution"
```

### 2. Hash Chain Tests

Implementations MUST compute hash chains identically.

#### 2.1 Reference Hash Computation

```yaml
test_id: hash.compute.single_event
description: Single event hash computation
input:
  trace_version: "1.0"
  event_id: "01234567-89ab-cdef-0123-456789abcdef"
  trace_id: "11111111-1111-1111-1111-111111111111"
  span_id: "22222222-2222-2222-2222-222222222222"
  parent_span_id: null
  session_id: "33333333-3333-3333-3333-333333333333"
  sequence: 0
  timestamp: "2024-12-28T12:00:00.000000Z"
  event_type: "session.started"
  payload:
    agent_id: "test-agent"
    goal: "Test session"
  previous_event_hash: "0000000000000000000000000000000000000000000000000000000000000000"
expected_hash: "a1b2c3d4e5f6..."  # Actual hash computed by reference implementation
```

#### 2.2 Chain Verification

```yaml
test_id: hash.chain.valid
description: Valid chain passes verification
input:
  - event_id: "event-1"
    sequence: 0
    event_hash: "hash1"
    previous_event_hash: "0000...0000"
  - event_id: "event-2"
    sequence: 1
    event_hash: "hash2"
    previous_event_hash: "hash1"
  - event_id: "event-3"
    sequence: 2
    event_hash: "hash3"
    previous_event_hash: "hash2"
expected: VALID
```

```yaml
test_id: hash.chain.broken
description: Broken chain fails verification
input:
  - event_id: "event-1"
    sequence: 0
    event_hash: "hash1"
    previous_event_hash: "0000...0000"
  - event_id: "event-2"
    sequence: 1
    event_hash: "hash2"
    previous_event_hash: "wrong_hash"  # Should be hash1
expected: INVALID
error: "chain_broken"
error_at: 1
```

```yaml
test_id: hash.chain.sequence_gap
description: Sequence gap fails verification
input:
  - event_id: "event-1"
    sequence: 0
    event_hash: "hash1"
    previous_event_hash: "0000...0000"
  - event_id: "event-2"
    sequence: 2  # Should be 1
    event_hash: "hash2"
    previous_event_hash: "hash1"
expected: INVALID
error: "sequence_gap"
error_at: 1
```

### 3. Policy Evaluation Tests

Implementations MUST evaluate policies in the correct order.

#### 3.1 Deny Takes Precedence

```yaml
test_id: policy.order.deny_precedence
description: Deny policy overrides allow
atlas:
  policies:
    - policy_id: "allow-all"
      type: "allow"
      actions: ["*"]
      priority: 0
    - policy_id: "deny-delete"
      type: "deny"
      actions: ["*.delete"]
      priority: 100
request:
  task:
    goal: "Delete resource"
  action: "resource.delete"
expected:
  decision: "deny"
  policy_id: "deny-delete"
```

#### 3.2 Rate Limit Enforcement

```yaml
test_id: policy.rate_limit.enforced
description: Rate limit blocks after threshold
atlas:
  policies:
    - policy_id: "rate-limit-api"
      type: "rate_limit"
      actions: ["api.*"]
      parameters:
        max_calls: 2
        window_seconds: 60
sequence:
  - action: "api.call"
    expected: "allow"
  - action: "api.call"
    expected: "allow"
  - action: "api.call"
    expected: "deny"
    reason: "rate_limit_exceeded"
```

#### 3.3 Risk Tier Escalation

```yaml
test_id: policy.risk_tier.escalation
description: High risk requires approval
atlas:
  policies:
    - policy_id: "high-risk-approval"
      type: "require_approval"
      conditions:
        risk_tiers: ["high", "critical"]
request:
  task:
    goal: "Delete production database"
    risk_tier: "high"
  action: "database.drop"
expected:
  decision: "requires_approval"
```

### 4. Resolution Tests

Implementations MUST produce consistent resolutions.

#### 4.1 Context Block Ordering

```yaml
test_id: resolution.context.ordering
description: Context blocks ordered by priority
atlas:
  context_packs:
    - pack_id: "low-priority"
      priority: 10
      files: ["low.md"]
    - pack_id: "high-priority"
      priority: 100
      files: ["high.md"]
    - pack_id: "medium-priority"
      priority: 50
      files: ["medium.md"]
request:
  task:
    goal: "Need context"
expected:
  context_blocks:
    - source_pack: "high-priority"
    - source_pack: "medium-priority"
    - source_pack: "low-priority"
```

#### 4.2 Action Filtering

```yaml
test_id: resolution.actions.capability_filter
description: Only requested capabilities returned
atlas:
  capabilities:
    - capability_id: "read"
      actions: ["resource.get", "resource.list"]
    - capability_id: "write"
      actions: ["resource.create", "resource.update"]
request:
  task:
    goal: "Read resources"
    required_capabilities: ["read"]
expected:
  allowed_actions:
    - action_id: "resource.get"
    - action_id: "resource.list"
  # write actions should NOT be included
```

### 5. Replay Tests

Implementations MUST support deterministic replay.

#### 5.1 Deterministic Resolution

```yaml
test_id: replay.deterministic
description: Same input produces same output
atlas_file: "reference-atlas.json"
trace_file: "reference-trace.jsonl"
verification:
  - replay trace with atlas
  - compare resolutions
  - must be identical (ignoring timestamps and IDs)
```

#### 5.2 Trace Diff

```yaml
test_id: replay.diff.semantic
description: Semantic diff identifies changes
trace_a:
  - event_type: "action.executed"
    payload:
      action_id: "resource.create"
      duration_ms: 100
trace_b:
  - event_type: "action.executed"
    payload:
      action_id: "resource.create"
      duration_ms: 150  # Different timing
expected_diff:
  compatibility: "semantically_equivalent"
  differences:
    - path: "/0/payload/duration_ms"
      type: "value_changed"
      significance: "minor"
```

### 6. Atlas Validation Tests

Implementations MUST validate Atlas manifests.

#### 6.1 Valid Manifest

```yaml
test_id: atlas.manifest.valid
description: Valid manifest passes validation
input:
  atlas_version: "1.0"
  atlas_id: "com.example.test"
  version: "1.0.0"
  name: "Test Atlas"
  description: "A test atlas"
  actions:
    - action_id: "test.action"
      name: "Test Action"
      description: "Does testing"
expected: VALID
```

#### 6.2 Invalid Atlas ID

```yaml
test_id: atlas.manifest.invalid_id
description: Invalid atlas ID fails validation
input:
  atlas_version: "1.0"
  atlas_id: "InvalidID"  # Must be lowercase with dots
  version: "1.0.0"
  name: "Test Atlas"
  description: "A test atlas"
expected: INVALID
error_path: "/atlas_id"
```

#### 6.3 Invalid Semver

```yaml
test_id: atlas.manifest.invalid_version
description: Invalid semver fails validation
input:
  atlas_version: "1.0"
  atlas_id: "com.example.test"
  version: "1.0"  # Must be full semver
  name: "Test Atlas"
  description: "A test atlas"
expected: INVALID
error_path: "/version"
```

---

## Golden Trace Tests

Golden traces are reference executions that implementations must match.

### Directory Structure

```
specs/conformance/golden/
├── simple-resolve/
│   ├── atlas.json
│   ├── request.json
│   ├── expected-resolution.json
│   └── expected-trace.jsonl
├── policy-deny/
│   ├── atlas.json
│   ├── request.json
│   ├── expected-resolution.json
│   └── expected-trace.jsonl
└── multi-action/
    ├── atlas.json
    ├── requests/
    │   ├── 001-resolve.json
    │   ├── 002-execute.json
    │   └── 003-execute.json
    ├── expected-resolutions/
    │   └── 001-resolution.json
    └── expected-trace.jsonl
```

### Comparison Rules

When comparing against golden traces:

1. **Ignore dynamic fields:**
   - `event_id`
   - `trace_id`
   - `span_id`
   - `request_id`
   - `resolution_id`
   - `execution_id`
   - `timestamp`
   - `event_hash`
   - `previous_event_hash`

2. **Must match exactly:**
   - `event_type`
   - `sequence`
   - `payload` structure
   - `decision` outcomes

3. **Semantic equivalence:**
   - Array order matters for `context_blocks`
   - Array order matters for `allowed_actions`
   - Object key order does not matter

---

## Running Conformance Tests

### Test Harness Requirements

Implementations MUST provide a test harness that:

1. Accepts JSON Schema test cases
2. Accepts golden trace directories
3. Reports pass/fail with details
4. Outputs machine-readable results

### Expected Output Format

```json
{
  "implementation": "cra-python",
  "version": "0.1.0",
  "conformance_level": "CRA-Full",
  "test_run": {
    "timestamp": "2024-12-28T12:00:00Z",
    "duration_ms": 1234,
    "total": 100,
    "passed": 98,
    "failed": 2,
    "skipped": 0
  },
  "results": [
    {
      "test_id": "schema.carp.request.valid",
      "status": "passed",
      "duration_ms": 5
    },
    {
      "test_id": "hash.chain.broken",
      "status": "failed",
      "duration_ms": 3,
      "error": "Expected INVALID, got VALID",
      "details": { ... }
    }
  ]
}
```

---

## Certification Process

1. Implementation runs full conformance suite
2. Results submitted with implementation source
3. Review by CRA maintainers
4. Certification badge issued for passing implementations

### Certification Levels

| Badge | Requirements |
|-------|--------------|
| 🥉 **CRA-Minimal** | Pass all core tests |
| 🥈 **CRA-Full** | Pass all tests including HTTP API |
| 🥇 **CRA-Certified** | Full + security audit + performance benchmarks |

---

*This specification is released under the MIT License.*
