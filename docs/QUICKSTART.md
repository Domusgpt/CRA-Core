# CRA Quick Start Guide

## What is CRA?

CRA (Context Registry Agents) is a governed context layer that makes AI agents use tools, systems, and proprietary knowledge **correctly**.

**Core Protocols:**
- **CARP** (Context & Action Resolution Protocol) - resolves what context and actions are permitted
- **TRACE** (Telemetry & Replay Artifact Contract Envelope) - proves what happened

**Key Principle:** If it wasn't emitted by the runtime, it didn't happen.

## Installation

```bash
# Clone the repository
git clone https://github.com/your-org/cra-core.git
cd cra-core

# Install dependencies
npm install

# Build all packages
npm run build

# Link CLI globally (optional)
npm link --workspace=packages/cli
```

## Initialize a Project

```bash
# Create project structure
cra init

# This creates:
# - agents.md (operational rules for AI agents)
# - config/cra.json (runtime configuration)
# - traces/ (telemetry output)
# - atlases/ (context packages)
```

## Load an Atlas

```bash
# Copy the reference atlas
cp -r atlases/github-ops atlases/

# Validate the atlas
cra atlas validate atlases/github-ops
```

## Resolve Context

```bash
# Ask CARP for context and permissions
cra resolve "Create a bug report for the login issue"

# Output:
# Resolution: 0192xxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
# Decision: allow
# Context blocks: 1
# Allowed actions: 5
# Policies applied: 1
```

## Watch Telemetry

```bash
# Tail the trace stream
cra trace tail

# Output:
# 12:00:00.123 info  session.started           {"session_id":"..."}
# 12:00:00.125 info  carp.request.received     {"goal":"..."}
# 12:00:00.130 info  atlas.load.completed      {"atlas_ref":"..."}
# 12:00:00.135 info  carp.resolution.completed {"decision_type":"allow"}
```

## Verify Trace Integrity

```bash
# Verify hash chain
cra trace verify traces/your-trace.jsonl

# Output:
# ✓ Trace integrity verified
#   Events: 15
#   Chain: intact
```

## Using with LLMs

### OpenAI Integration

```typescript
import { CRARuntime, createRequest } from '@cra/runtime';
import { OpenAIAdapter } from '@cra/adapters';
import OpenAI from 'openai';

// Initialize CRA
const runtime = new CRARuntime();
await runtime.loadAtlas('./atlases/github-ops');

// Get resolution
const resolution = await runtime.resolve(createRequest('resolve', {
  agent_id: 'my-agent',
  session_id: 'session-1',
}, {
  task: {
    goal: 'Create a bug report',
    risk_tier: 'low',
    context_hints: ['github.issues'],
  },
}));

// Convert to OpenAI format
const adapter = new OpenAIAdapter({ platform: 'openai' });
const { systemPrompt, tools } = adapter.getConfiguration(resolution);

// Use with OpenAI
const openai = new OpenAI();
const response = await openai.chat.completions.create({
  model: 'gpt-4',
  messages: [
    { role: 'system', content: systemPrompt },
    { role: 'user', content: 'Create a bug report for the login issue' },
  ],
  tools: adapter.toOpenAITools(resolution.allowed_actions),
});
```

### Claude Integration

```typescript
import { CRARuntime, createRequest } from '@cra/runtime';
import { ClaudeAdapter } from '@cra/adapters';
import Anthropic from '@anthropic-ai/sdk';

// Initialize CRA
const runtime = new CRARuntime();
await runtime.loadAtlas('./atlases/github-ops');

// Get resolution
const resolution = await runtime.resolve(/* ... */);

// Convert to Claude format
const adapter = new ClaudeAdapter({ platform: 'claude' });
const { systemPrompt, tools } = adapter.getConfiguration(resolution);

// Use with Claude
const client = new Anthropic();
const response = await client.messages.create({
  model: 'claude-3-opus-20240229',
  max_tokens: 1024,
  system: systemPrompt,
  tools: adapter.toClaudeTools(resolution.allowed_actions),
  messages: [
    { role: 'user', content: 'Create a bug report for the login issue' },
  ],
});
```

## Configuration

Edit `config/cra.json`:

```json
{
  "version": "0.1",
  "runtime": {
    "trace_dir": "./traces",
    "trace_to_file": true,
    "default_ttl_seconds": 300,
    "max_context_tokens": 8192,
    "max_actions_per_resolution": 50
  },
  "atlases": {
    "paths": ["./atlases"],
    "auto_load": true
  },
  "telemetry": {
    "format": "jsonl",
    "console_output": true
  }
}
```

## Next Steps

1. **Create an Atlas** - Package your domain expertise
2. **Define Policies** - Set up governance rules
3. **Add Adapters** - Support your LLM platform
4. **Write Tests** - Use golden traces for regression

See the full documentation:
- [Architecture](./ARCHITECTURE.md)
- [CARP Specification](./specs/CARP_SPEC.md)
- [TRACE Specification](./specs/TRACE_SPEC.md)
- [Testing Guide](./TESTING.md)
