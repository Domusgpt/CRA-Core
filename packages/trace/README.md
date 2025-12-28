# @cra/trace

TRACE (Telemetry & Replay Artifact Contract Envelope) collector and utilities for CRA.

## Features

- Append-only event collection with hash chain integrity
- Span tracking and correlation
- File output in JSONL format
- Redaction engine for sensitive data
- Golden trace testing framework
- Trace replay and diff utilities

## Installation

```bash
npm install @cra/trace
```

## Usage

### Basic Collector

```typescript
import { TRACECollector } from '@cra/trace';

const collector = new TRACECollector({
  session_id: 'session-1',
  trace_id: 'trace-1',
  output_dir: './traces',
  file_output: true,
});

// Record events
collector.record('session.started', { agent: 'my-agent' });
collector.record('carp.request.received', { goal: 'Create issue' });
collector.record('carp.resolution.completed', { decision: 'allow' });

// Get all events
const events = collector.getEvents();

// Close and flush
await collector.close();
```

### Event Listener

```typescript
collector.on('event', (event) => {
  console.log(`[${event.event_type}]`, event.payload);
});
```

### Span Tracking

```typescript
// Start a span
const span = collector.startSpan('carp.resolve', {
  request_id: 'req-123',
});

// Record events within span
collector.record('carp.context.loaded', { blocks: 5 });
collector.record('carp.policy.evaluated', { rules: 3 });

// End span
collector.endSpan(span.span_id, { status: 'ok' });
```

## Redaction Engine

Protect sensitive data in TRACE events.

### Basic Usage

```typescript
import { RedactionEngine } from '@cra/trace';

const engine = new RedactionEngine({
  enabled: true,
  patterns: [
    { name: 'email', pattern: /\b[\w.-]+@[\w.-]+\.\w+\b/gi, replacement: '[EMAIL]' },
    { name: 'ssn', pattern: /\b\d{3}-\d{2}-\d{4}\b/g, replacement: '[SSN]' },
  ],
});

// Redact a TRACE event
const redactedEvent = engine.redactEvent(event);

// Redact arbitrary data
const redactedData = engine.redactObject({
  email: 'user@example.com',  // becomes '[EMAIL]'
  ssn: '123-45-6789',         // becomes '[SSN]'
});
```

### Pre-configured Security Engine

```typescript
import { RedactionEngine } from '@cra/trace';

// Includes all common patterns: email, phone, SSN, credit card, API keys, JWT, IP, password
const engine = RedactionEngine.createSecurityEngine();

const redacted = engine.redactString('Contact: user@example.com, SSN: 123-45-6789');
// "Contact: [EMAIL REDACTED], SSN: [SSN REDACTED]"
```

### Field-Level Rules

```typescript
const engine = new RedactionEngine({
  enabled: true,
  fieldRules: [
    { field: 'password', mode: 'remove' },        // Remove entirely
    { field: 'api_key', mode: 'hash' },           // SHA-256 hash
    { field: 'phone', mode: 'partial', keep: 4 }, // Keep last 4 digits
    { field: 'email', mode: 'mask', char: '*' },  // Mask with asterisks
  ],
});
```

### Redaction Modes

| Mode | Description | Example |
|------|-------------|---------|
| `full` | Replace entirely | `[REDACTED]` |
| `partial` | Keep some characters | `***-**-6789` |
| `hash` | SHA-256 hash | `a1b2c3...` |
| `mask` | Replace with character | `******` |
| `remove` | Remove field entirely | (field deleted) |

## Golden Trace Testing

Record and validate trace sequences for regression testing.

### Recording a Golden Trace

```typescript
import { createGoldenTraceManager, TRACECollector } from '@cra/trace';

const manager = createGoldenTraceManager();
const collector = new TRACECollector({ session_id: 'test' });

// Start recording
manager.startRecording(collector, 'login-flow');

// Run your operations
collector.record('session.started', {});
collector.record('carp.resolution.completed', { decision: 'allow' });

// Stop and get golden trace
const golden = manager.stopRecording();

// Export for later use
const json = manager.exportGolden('login-flow');
fs.writeFileSync('golden/login-flow.json', json);
```

### Comparing Against Golden

```typescript
// Load golden traces
const goldens = JSON.parse(fs.readFileSync('golden/login-flow.json'));
manager.loadGoldens([goldens]);

// Run test and capture events
const capturedEvents = [...]; // Events from test run

// Compare
const result = manager.compare('login-flow', capturedEvents);

if (result.passed) {
  console.log('Test passed!');
} else {
  console.log('Differences:', result.differences);
}
```

### Test Framework Integration

```typescript
import { describe, it, expect } from 'vitest';
import { createGoldenTraceManager, TRACECollector } from '@cra/trace';

describe('Login Flow', () => {
  it('should match golden trace', () => {
    const manager = createGoldenTraceManager();
    const collector = new TRACECollector({ session_id: 'test' });
    const assertion = manager.createAssertion(collector);

    // Load golden
    manager.registerGolden('login', goldenTrace);

    // Run operations
    collector.record('session.started', {});
    collector.record('auth.completed', { user: 'test' });

    // Assert
    const result = assertion.matchesGolden('login');
    expect(result.passed).toBe(true);
  });
});
```

### Configuration

```typescript
const manager = createGoldenTraceManager({
  ignoreFields: ['event_id', 'timestamp', 'event_hash'],
  ignoreTimestamps: true,
  ignoreHashes: true,
  numericTolerance: 0.01,  // For floating point comparisons
  maxDifferences: 0,        // Fail on any difference
});
```

## Trace Utilities

### Load and Replay

```typescript
import { loadTraceFile, replayTrace } from '@cra/trace';

// Load from JSONL file
const events = await loadTraceFile('./traces/session-1.jsonl');

// Replay with timing
for await (const event of replayTrace(events, { speed: 2.0 })) {
  console.log(event.event_type);
}
```

### Verify Hash Chain

```typescript
import { verifyChain } from '@cra/protocol';

const events = collector.getEvents();
const { valid, errors } = verifyChain(events);

if (!valid) {
  console.error('Chain integrity failed:', errors);
}
```

### Diff Traces

```typescript
import { diffTraces } from '@cra/protocol';

const diff = diffTraces(expectedEvents, actualEvents, {
  ignore_fields: ['event_id', 'timestamp'],
});

console.log('Compatibility:', diff.compatibility);
console.log('Differences:', diff.differences);
```

## License

MIT
