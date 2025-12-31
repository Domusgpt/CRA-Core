# CRA Checkpoint System Specification

## Overview

Checkpoints are moments when CRA intervenes—injecting context, checking policies, or recording significant events. The system is designed to be lean (not every prompt) and extensible (custom checkpoint types).

---

## Checkpoint Types

### Built-in Types

| Type | Trigger | Purpose |
|------|---------|---------|
| `session_start` | Session begins | Initial context injection |
| `session_end` | Session ends | Finalize TRACE |
| `keyword_match` | Keywords in input | Relevant context injection |
| `action_pre` | Before action execution | Policy check, context for action |
| `action_post` | After action execution | Record result |
| `risk_threshold` | Risk tier exceeded | Additional verification |
| `time_interval` | Elapsed time | Periodic refresh |
| `count_interval` | Action count | Periodic check |
| `explicit_request` | Agent asks | On-demand context |
| `error_occurred` | Error detected | Error context/guidance |

### Custom Types (via plugins)

Plugins can register additional checkpoint types:

```typescript
wrapper.registerCheckpoint('custom_checkpoint', {
  description: 'Triggered on custom condition',
  evaluator: (context) => boolean,
  handler: async (context) => CheckpointResult,
});
```

---

## Checkpoint Evaluation

### Evaluation Flow

```
Input/Event arrives
        │
        ▼
┌─────────────────────────────────────┐
│     Checkpoint Evaluator            │
│                                     │
│  For each registered checkpoint:    │
│  1. Check trigger condition         │
│  2. If triggered, add to queue      │
│                                     │
│  Sort queue by priority             │
│  Execute checkpoints in order       │
│                                     │
└─────────────────────────────────────┘
        │
        ▼
  Checkpoint handlers execute
        │
        ▼
  Continue with (possibly modified) input
```

### Trigger Conditions

```typescript
interface CheckpointTrigger {
  type: CheckpointType;
  condition: TriggerCondition;
  priority: number;  // Higher = runs first
}

type TriggerCondition =
  | { type: 'always' }
  | { type: 'keyword'; keywords: string[]; matchMode: 'any' | 'all' }
  | { type: 'regex'; pattern: string }
  | { type: 'action'; actions: string[] }
  | { type: 'risk'; minTier: RiskTier }
  | { type: 'interval'; seconds: number }
  | { type: 'count'; actions: number }
  | { type: 'custom'; evaluator: (ctx: EvalContext) => boolean };
```

---

## Checkpoint Configuration

### Atlas-Level Configuration

```json
{
  "checkpoint_config": {
    "session_start": {
      "enabled": true,
      "inject_contexts": ["essential-facts", "governance-rules"]
    },

    "keyword_match": {
      "enabled": true,
      "keywords": {
        "geometry": ["vib3-geometry-system"],
        "deploy|production": ["deployment-checklist", "safety-rules"],
        "delete|remove": ["destructive-action-warning"]
      },
      "match_mode": "any",
      "case_sensitive": false
    },

    "action_pre": {
      "enabled": true,
      "actions": {
        "write_file": {
          "inject_contexts": ["file-writing-guidelines"],
          "require_policy_check": false
        },
        "execute_code": {
          "inject_contexts": ["code-execution-safety"],
          "require_policy_check": true
        },
        "delete_*": {
          "inject_contexts": ["destructive-action-warning"],
          "require_policy_check": true,
          "require_confirmation": true
        }
      }
    },

    "risk_threshold": {
      "enabled": true,
      "min_tier": "high",
      "inject_contexts": ["high-risk-procedures"],
      "require_sync": true
    },

    "time_interval": {
      "enabled": false,
      "seconds": 300
    },

    "count_interval": {
      "enabled": false,
      "actions": 50
    }
  }
}
```

### Context Block Checkpoint Settings

Individual context blocks can specify when to inject:

```json
{
  "context_id": "vib3-geometry-system",
  "name": "Geometry System",
  "inject_mode": "on_match",
  "checkpoint_config": {
    "keywords": ["geometry", "index", "formula", "base", "core"],
    "actions": ["calculate_*", "set_geometry"],
    "priority": 380
  },
  "content": "..."
}
```

---

## Checkpoint Handlers

### Handler Interface

```typescript
interface CheckpointHandler {
  // Called when checkpoint triggers
  handle(context: CheckpointContext): Promise<CheckpointResult>;
}

interface CheckpointContext {
  type: CheckpointType;
  trigger: TriggerCondition;
  input?: AgentInput;
  action?: Action;
  session: SessionState;
  cache: ContextCache;
  client: CRAClient;
}

interface CheckpointResult {
  // Context to inject
  injectContext?: string[];

  // Modified input (if applicable)
  modifiedInput?: AgentInput;

  // Policy decision (if applicable)
  policyDecision?: PolicyDecision;

  // TRACE events to record
  traceEvents?: TraceEvent[];

  // Whether to continue or block
  continue: boolean;
  blockReason?: string;
}
```

### Built-in Handlers

#### Session Start Handler

```typescript
const sessionStartHandler: CheckpointHandler = {
  async handle(ctx) {
    // Fetch initial contexts marked as "always"
    const contexts = await ctx.client.requestContext(
      ctx.session.intent,
      { injectMode: 'always' }
    );

    return {
      injectContext: contexts.map(c => c.content),
      traceEvents: [{
        eventType: 'session_started',
        payload: { contextsInjected: contexts.map(c => c.id) }
      }],
      continue: true,
    };
  }
};
```

#### Keyword Match Handler

```typescript
const keywordMatchHandler: CheckpointHandler = {
  async handle(ctx) {
    const input = ctx.input?.content || '';
    const matchedKeywords = findKeywordMatches(input, ctx.trigger.keywords);

    if (matchedKeywords.length === 0) {
      return { continue: true };
    }

    // Fetch contexts for matched keywords
    const contexts = await ctx.client.requestContext(
      input,
      { hints: matchedKeywords }
    );

    return {
      injectContext: contexts.map(c => c.content),
      traceEvents: [{
        eventType: 'context_injected',
        payload: {
          trigger: 'keyword_match',
          keywords: matchedKeywords,
          contexts: contexts.map(c => c.id)
        }
      }],
      continue: true,
    };
  }
};
```

#### Action Pre Handler

```typescript
const actionPreHandler: CheckpointHandler = {
  async handle(ctx) {
    const action = ctx.action!;
    const config = getActionConfig(action.type, ctx.session.atlasConfig);

    // Get relevant context
    const contexts = config.injectContexts
      ? await ctx.client.getContexts(config.injectContexts)
      : [];

    // Check policy if required
    let policyDecision: PolicyDecision | undefined;
    if (config.requirePolicyCheck) {
      policyDecision = await ctx.client.checkPolicy(action);
    }

    // Record to TRACE
    const traceEvents: TraceEvent[] = [{
      eventType: 'action_attempted',
      payload: {
        action: action.type,
        params: action.params,
        policyDecision: policyDecision?.decision,
      }
    }];

    // Block if policy denied
    if (policyDecision?.decision === 'denied') {
      return {
        policyDecision,
        traceEvents,
        continue: false,
        blockReason: policyDecision.reason,
      };
    }

    return {
      injectContext: contexts.map(c => c.content),
      policyDecision,
      traceEvents,
      continue: true,
    };
  }
};
```

---

## Keyword Matching

### Matching Modes

| Mode | Description |
|------|-------------|
| `any` | Match if ANY keyword found |
| `all` | Match only if ALL keywords found |
| `phrase` | Match exact phrase |
| `regex` | Use regex pattern |
| `fuzzy` | Fuzzy matching (typo-tolerant) |

### Matching Algorithm

```typescript
function matchKeywords(
  input: string,
  keywords: string[],
  mode: MatchMode,
  options: MatchOptions
): MatchResult {
  const normalizedInput = options.caseSensitive
    ? input
    : input.toLowerCase();

  const matches: KeywordMatch[] = [];

  for (const keyword of keywords) {
    const normalizedKeyword = options.caseSensitive
      ? keyword
      : keyword.toLowerCase();

    let found = false;
    let position = -1;

    switch (mode) {
      case 'any':
      case 'all':
        position = normalizedInput.indexOf(normalizedKeyword);
        found = position !== -1;
        break;

      case 'phrase':
        const phraseRegex = new RegExp(`\\b${escapeRegex(normalizedKeyword)}\\b`);
        const phraseMatch = normalizedInput.match(phraseRegex);
        found = phraseMatch !== null;
        position = phraseMatch?.index ?? -1;
        break;

      case 'regex':
        const regex = new RegExp(keyword, options.caseSensitive ? '' : 'i');
        const regexMatch = normalizedInput.match(regex);
        found = regexMatch !== null;
        position = regexMatch?.index ?? -1;
        break;

      case 'fuzzy':
        // Levenshtein distance or similar
        found = fuzzyMatch(normalizedInput, normalizedKeyword, options.threshold);
        break;
    }

    if (found) {
      matches.push({ keyword, position });
    }
  }

  const triggered = mode === 'all'
    ? matches.length === keywords.length
    : matches.length > 0;

  return { triggered, matches };
}
```

---

## Risk Tier Detection

### Risk Tier Levels

| Tier | Description | Examples |
|------|-------------|----------|
| `low` | Read-only, reversible | Read file, search |
| `medium` | Writes, but recoverable | Write file, create resource |
| `high` | Destructive, hard to reverse | Delete, modify production |
| `critical` | Irreversible, high impact | Drop database, deploy |

### Detection Logic

```typescript
function detectRiskTier(action: Action): RiskTier {
  // Check explicit risk tier in atlas action definition
  const actionDef = getActionDefinition(action.type);
  if (actionDef?.riskTier) {
    return actionDef.riskTier;
  }

  // Pattern-based detection
  const patterns: [RegExp, RiskTier][] = [
    [/^(read|get|list|search|find)/, 'low'],
    [/^(write|create|update|set)/, 'medium'],
    [/^(delete|remove|drop|destroy)/, 'high'],
    [/^(deploy|migrate|truncate)/, 'critical'],
    [/production|prod/, 'high'],
  ];

  for (const [pattern, tier] of patterns) {
    if (pattern.test(action.type) || pattern.test(JSON.stringify(action.params))) {
      return tier;
    }
  }

  return 'low';  // Default
}
```

---

## Checkpoint Priority

When multiple checkpoints trigger, they execute in priority order:

| Priority | Checkpoint Type | Reason |
|----------|-----------------|--------|
| 1000 | `session_start` | Must run first |
| 900 | `risk_threshold` | Security first |
| 800 | `action_pre` (policy check) | Block before context |
| 700 | `action_pre` (context) | Context for action |
| 600 | `keyword_match` | Relevant context |
| 500 | `time_interval` | Periodic refresh |
| 400 | `count_interval` | Periodic check |
| 300 | `explicit_request` | On-demand |
| 200 | `custom` | Plugin checkpoints |
| 100 | `action_post` | Record after |
| 0 | `session_end` | Must run last |

---

## Checkpoint Caching

### What Gets Cached

- Context block content (by context ID)
- Policy decisions (by action + params hash)
- Keyword match results (by input hash, short TTL)

### Cache Invalidation

Checkpoints can invalidate cache:

```typescript
interface CheckpointResult {
  // ... other fields ...

  // Cache operations
  cacheInvalidate?: string[];  // Keys to invalidate
  cacheClear?: boolean;        // Clear entire cache
}
```

### TTL Configuration

```json
{
  "cache_config": {
    "context_ttl_seconds": 300,
    "policy_ttl_seconds": 60,
    "keyword_ttl_seconds": 10
  }
}
```

---

## TRACE Events from Checkpoints

Checkpoints generate TRACE events:

| Checkpoint | Event Type |
|------------|------------|
| `session_start` | `session_started` |
| `session_end` | `session_ended` |
| `keyword_match` | `context_injected` |
| `action_pre` | `action_attempted` |
| `action_post` | `action_completed` |
| `risk_threshold` | `risk_checkpoint` |
| `error_occurred` | `error_handled` |

---

## Extensibility

### Custom Checkpoint Types

```typescript
// Register new checkpoint type
cra.registerCheckpointType('sentiment_negative', {
  description: 'Triggered when input sentiment is negative',

  // Evaluation function
  evaluate: async (input: string) => {
    const sentiment = await analyzeSentiment(input);
    return sentiment.score < -0.5;
  },

  // Default handler
  defaultHandler: async (ctx) => {
    return {
      injectContext: ['handling-negative-feedback'],
      continue: true,
    };
  },

  // Default priority
  defaultPriority: 650,
});
```

### Checkpoint Plugins

```typescript
interface CheckpointPlugin {
  name: string;

  // Register custom checkpoint types
  checkpointTypes?: CheckpointTypeDefinition[];

  // Add handlers for existing types
  handlers?: Record<CheckpointType, CheckpointHandler>;

  // Modify checkpoint config
  configureCheckpoints?: (config: CheckpointConfig) => CheckpointConfig;
}
```

### External Integrations

Checkpoints can call external services:

```typescript
const externalValidationHandler: CheckpointHandler = {
  async handle(ctx) {
    // Call external validation API
    const result = await fetch('https://validation.example.com/check', {
      method: 'POST',
      body: JSON.stringify({
        action: ctx.action,
        session: ctx.session.id,
      }),
    });

    const validation = await result.json();

    return {
      continue: validation.approved,
      blockReason: validation.reason,
      traceEvents: [{
        eventType: 'external_validation',
        payload: validation,
      }],
    };
  }
};
```

---

## Performance Considerations

### Minimize Checkpoint Overhead

1. **Keyword matching is fast** - Simple string operations
2. **Cache aggressively** - Don't re-fetch same context
3. **Async when possible** - Don't block for TRACE
4. **Batch requests** - Combine context requests

### Checkpoint Budgets

Optional budget limits:

```json
{
  "checkpoint_config": {
    "budget": {
      "max_checkpoints_per_input": 5,
      "max_context_injection_size": 10000,
      "max_checkpoint_time_ms": 500
    }
  }
}
```

---

## Summary

Checkpoints are when CRA intervenes. They are:

- **Configurable** - Atlas defines when and how
- **Prioritized** - Execute in defined order
- **Cached** - Avoid redundant work
- **Extensible** - Custom types via plugins
- **Lean** - Not every prompt, only when needed

Built-in types handle common cases. Plugins extend for specific needs. The system scales from minimal (just session start/end) to comprehensive (every action checked).
