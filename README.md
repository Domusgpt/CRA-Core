# CRA — Context Registry Agents

> **The context authority layer that makes agents reliable, governable, and provable.**

CRA is a protocol-first infrastructure for agentic systems. It provides governed, platform-aware context and permitted actions to LLM agents through two foundational protocols:

- **CARP** (Context & Action Resolution Protocol) — resolves what context and actions are permitted
- **TRACE** (Telemetry & Replay Artifact Contract Envelope) — proves what happened

## Key Principle

> **If it wasn't emitted by the runtime, it didn't happen.**

LLM narration is advisory. TRACE events are authoritative.

## Quick Start

```bash
# Install dependencies
npm install

# Build all packages
npm run build

# Initialize a project
npx cra init

# Load an atlas and resolve context
npx cra resolve "Create a bug report for the login issue"

# Watch telemetry
npx cra trace tail
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              CRA SYSTEM                                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────────┐     CARP Request      ┌────────────────────────────┐ │
│  │  LLM Agent   │ ────────────────────► │      CRA Runtime           │ │
│  │  (Requester) │ ◄──────────────────── │   (Resolver + Policy)      │ │
│  └──────────────┘     CARP Resolution   └─────────────┬──────────────┘ │
│         │                                             │                 │
│         │                                    Load     │                 │
│         ▼                                             ▼                 │
│  ┌──────────────┐                        ┌────────────────────────────┐│
│  │   Platform   │                        │       Atlas Store          ││
│  │   Adapter    │                        │  (Context + Policies +     ││
│  │(OpenAI/Claude│                        │   Adapters + Tests)        ││
│  │   /MCP)      │                        └────────────────────────────┘│
│  └──────────────┘                                                       │
│         │                                                               │
│         │ TRACE Events                                                  │
│         ▼                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐│
│  │                      TRACE Collector                                 ││
│  │           (Append-only + Hash Chain + JSONL Stream)                 ││
│  └─────────────────────────────────────────────────────────────────────┘│
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

## Packages

| Package | Description |
|---------|-------------|
| `@cra/protocol` | CARP and TRACE type definitions and utilities |
| `@cra/trace` | TRACE collector with hash chain integrity |
| `@cra/atlas` | Atlas loader and validator |
| `@cra/runtime` | CRA runtime with CARP resolver |
| `@cra/adapters` | Platform adapters (OpenAI, Claude, MCP) |
| `@cra/cli` | Command-line interface |

## Core Concepts

### CARP Resolution

```typescript
import { CRARuntime, createRequest } from '@cra/runtime';

const runtime = new CRARuntime();
await runtime.loadAtlas('./atlases/github-ops');

const request = createRequest('resolve', {
  agent_id: 'my-agent',
  session_id: 'session-1',
}, {
  task: {
    goal: 'Create a bug report for the login issue',
    risk_tier: 'low',
    context_hints: ['github.issues'],
  },
});

const resolution = await runtime.resolve(request);
// resolution.decision.type === 'allow'
// resolution.context_blocks: relevant context
// resolution.allowed_actions: permitted actions
```

### Platform Adapters

```typescript
import { ClaudeAdapter } from '@cra/adapters';

const adapter = new ClaudeAdapter({ platform: 'claude' });
const { systemPrompt, tools } = adapter.getConfiguration(resolution);

// Use with Claude API
const response = await anthropic.messages.create({
  model: 'claude-3-opus-20240229',
  system: systemPrompt,
  tools: adapter.toClaudeTools(resolution.allowed_actions),
  messages: [{ role: 'user', content: userMessage }],
});
```

### TRACE Telemetry

```typescript
import { TRACECollector, verifyChain } from '@cra/trace';

// Events are emitted automatically
runtime.getTrace().on('event', (event) => {
  console.log(event.event_type, event.payload);
});

// Verify integrity
const events = runtime.getTrace().getEvents();
const { valid, errors } = verifyChain(events);
```

## Atlas Format

Atlases are versioned packages of domain expertise:

```
atlases/github-ops/
├── atlas.json          # Manifest
├── context/
│   ├── issues-guide.md # Context packs
│   └── pr-guide.md
├── adapters/
│   ├── openai.json     # Platform configs
│   ├── claude.json
│   └── mcp.json
└── tests/
    └── conformance.test.ts
```

Example `atlas.json`:

```json
{
  "atlas_version": "0.1",
  "metadata": {
    "id": "com.example.github-ops",
    "name": "GitHub Operations",
    "version": "0.1.0"
  },
  "domains": [
    { "id": "github.issues", "name": "Issues", "risk_tier": "low" }
  ],
  "context_packs": [
    { "id": "issues-guide", "domain": "github.issues", "source": "context/issues-guide.md" }
  ],
  "policies": [
    { "id": "safety", "rules": [/* ... */] }
  ],
  "actions": [
    { "id": "create-issue", "type": "api.github.create_issue", /* ... */ }
  ]
}
```

## CLI Commands

```bash
cra init                    # Initialize project
cra resolve <goal>          # Resolve context for a goal
cra execute <action>        # Execute a permitted action
cra trace tail              # Tail TRACE events
cra trace replay <file>     # Replay a trace
cra trace diff <a> <b>      # Compare traces
cra trace verify <file>     # Verify integrity
cra atlas validate <path>   # Validate an atlas
cra atlas list              # List installed atlases
```

## Documentation

- [Quick Start Guide](./docs/QUICKSTART.md)
- [Architecture](./docs/ARCHITECTURE.md)
- [CARP Specification](./docs/specs/CARP_SPEC.md)
- [TRACE Specification](./docs/specs/TRACE_SPEC.md)
- [Testing Guide](./docs/TESTING.md)
- [Roadmap](./docs/ROADMAP.md)

## What Makes CRA Unique

1. **Context is governed** — TTL-bounded, evidence-linked, redactable
2. **Runtime is authoritative** — LLM output is advisory, not truth
3. **Telemetry is replayable** — Hash-chained, diffable, audit-grade
4. **One Atlas, many platforms** — Works with OpenAI, Claude, MCP, and more

## Business Model

Creators publish Atlases to a marketplace:
- Free, paid, subscription, or usage-metered
- Platform certification for CARP/TRACE compliance
- Revenue sharing for creators

## Development

```bash
# Install dependencies
npm install

# Build all packages
npm run build

# Run tests
npm test

# Type check
npm run typecheck
```

## License

MIT

---

**"The context authority layer that makes agents reliable, governable, and provable."**
