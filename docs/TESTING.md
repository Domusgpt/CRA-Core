# CRA Testing Strategy

## Overview

CRA uses a multi-layered testing approach to ensure protocol conformance, runtime correctness, and regression detection.

## Test Categories

### 1. Unit Tests

Standard unit tests for individual modules.

```bash
# Run all unit tests
npm test

# Run tests for specific package
npm test --workspace=packages/protocol
```

### 2. Conformance Tests

Verify implementations conform to CARP/1.0 and TRACE/1.0 specifications.

#### CARP Conformance

| Test ID | Description | Requirement |
|---------|-------------|-------------|
| CARP-001 | Request includes carp_version | MUST |
| CARP-002 | Request IDs are UUIDv7 | MUST |
| CARP-003 | Timestamps are ISO 8601 | MUST |
| CARP-004 | Resolution includes all required fields | MUST |
| CARP-005 | TTL values are enforced | MUST |
| CARP-006 | Content hashes are SHA-256 | MUST |
| CARP-007 | Policies evaluated in priority order | MUST |
| CARP-008 | Resolution caching respects TTL | SHOULD |
| CARP-009 | Evidence included for context blocks | SHOULD |

#### TRACE Conformance

| Test ID | Description | Requirement |
|---------|-------------|-------------|
| TRACE-001 | Events include trace_version | MUST |
| TRACE-002 | Event IDs are UUIDv7 | MUST |
| TRACE-003 | Sequence numbers are monotonic | MUST |
| TRACE-004 | Event hashes are correct | MUST |
| TRACE-005 | Hash chain is maintained | MUST |
| TRACE-006 | JSONL format is valid | MUST |
| TRACE-007 | Spans have proper hierarchy | SHOULD |

### 3. Golden Trace Tests

Compare actual trace output against expected "golden" traces for regression detection.

#### Creating Golden Traces

```bash
# Generate a golden trace
cra resolve "Create a GitHub issue" --trace > tests/golden/create-issue.trace.jsonl

# Mark as golden
mv tests/golden/create-issue.trace.jsonl tests/golden/create-issue.golden.jsonl
```

#### Running Golden Tests

```bash
# Run against golden traces
cra test golden tests/golden/

# With specific test
cra test golden tests/golden/create-issue.golden.jsonl
```

#### Golden Trace Format

```json
{
  "name": "create-issue-resolution",
  "description": "Resolving context for issue creation",
  "golden_trace": "tests/golden/create-issue.golden.jsonl",
  "input": {
    "goal": "Create a GitHub issue for the login bug",
    "risk_tier": "low",
    "context_hints": ["github.issues"]
  },
  "comparison": {
    "ignore_fields": ["event_id", "timestamp", "event_hash", "sequence"],
    "ignore_event_types": [],
    "allow_additional_events": false,
    "artifact_comparison": "hash"
  },
  "assertions": [
    {
      "path": "events[?event_type=='carp.resolution.completed'].payload.decision_type",
      "operator": "eq",
      "value": "allow",
      "message": "Resolution should be allowed"
    }
  ]
}
```

#### Updating Golden Traces

When intentional changes require updating golden traces:

```bash
# Update specific golden trace
cra test golden --update tests/golden/create-issue.golden.jsonl

# Review changes
cra trace diff old.golden.jsonl new.golden.jsonl
```

### 4. Integration Tests

End-to-end tests that verify the complete flow.

```typescript
import { CRARuntime, createRequest } from '@cra/runtime';

describe('CRA Integration', () => {
  let runtime: CRARuntime;

  beforeAll(async () => {
    runtime = new CRARuntime();
    await runtime.loadAtlas('./atlases/github-ops');
  });

  afterAll(async () => {
    await runtime.shutdown();
  });

  it('should resolve a GitHub issue creation request', async () => {
    const request = createRequest('resolve', {
      agent_id: 'test',
      session_id: 'test-session',
    }, {
      task: {
        goal: 'Create a bug report issue',
        risk_tier: 'low',
        context_hints: ['github.issues'],
      },
    });

    const result = await runtime.resolve(request);

    expect('resolution_id' in result).toBe(true);
    if ('resolution_id' in result) {
      expect(result.decision.type).toBe('allow');
      expect(result.context_blocks.length).toBeGreaterThan(0);
      expect(result.allowed_actions.length).toBeGreaterThan(0);
    }
  });
});
```

## Test Utilities

### Trace Verification

```typescript
import { verifyChain, loadTraceFile } from '@cra/trace';

// Load and verify a trace file
const events = await loadTraceFile('./traces/session.trace.jsonl');
const { valid, errors } = verifyChain(events);

if (!valid) {
  console.error('Chain verification failed:', errors);
}
```

### Trace Diffing

```typescript
import { diffTraces, loadTraceFile } from '@cra/trace';

const expected = await loadTraceFile('expected.jsonl');
const actual = await loadTraceFile('actual.jsonl');

const diff = diffTraces(expected, actual, {
  ignore_fields: ['timestamp', 'event_id'],
});

console.log('Compatibility:', diff.compatibility);
console.log('Differences:', diff.differences);
```

### Request Validation

```typescript
import { validateRequest } from '@cra/protocol';

const request = { /* ... */ };
const { valid, errors } = validateRequest(request);

if (!valid) {
  console.error('Invalid request:', errors);
}
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: CRA Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install dependencies
        run: npm ci

      - name: Build
        run: npm run build

      - name: Run unit tests
        run: npm test

      - name: Run conformance tests
        run: npm run test:conformance

      - name: Run golden trace tests
        run: npm run test:golden

      - name: Upload trace artifacts
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: failed-traces
          path: traces/
```

## Best Practices

### 1. Trace-Driven Development

Write golden traces for expected behavior before implementation:

1. Define the expected CARP resolution
2. Define the expected TRACE events
3. Implement the feature
4. Compare actual output to golden

### 2. Minimal Golden Traces

Keep golden traces focused on essential events:

- Use `ignore_fields` for non-deterministic values
- Use `ignore_event_types` for verbose events
- Focus assertions on critical behavior

### 3. Regression Prevention

Run golden trace tests on every PR:

- Unexpected changes fail the build
- Intentional changes require explicit `--update`
- All updates require review

### 4. Trace Archival

Archive traces for important sessions:

```bash
# Archive current session
cra trace archive --session <id> --output archives/

# List archived traces
cra trace list --archives
```

## Troubleshooting

### Common Issues

**Hash chain verification fails**
- Check for event modification after emission
- Verify events are written in order

**Golden trace comparison fails unexpectedly**
- Check if new events were added
- Review `ignore_fields` configuration

**Conformance test timeouts**
- Check atlas loading time
- Verify network connectivity for remote resources
