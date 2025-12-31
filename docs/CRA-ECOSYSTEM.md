# CRA Ecosystem & Extensibility

## Overview

CRA is designed as an extensible platform. This document outlines the ecosystem components: plugins, extensions, marketplace, skills, and the atlas creator platform.

---

## Ecosystem Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         CRA ECOSYSTEM                                    │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    ATLAS MARKETPLACE                             │   │
│  │                                                                   │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐            │   │
│  │  │ Atlas A │  │ Atlas B │  │ Atlas C │  │ Atlas D │   ...      │   │
│  │  └─────────┘  └─────────┘  └─────────┘  └─────────┘            │   │
│  │                                                                   │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                    │                                     │
│  ┌─────────────────────────────────┼─────────────────────────────────┐ │
│  │                    PLUGIN REGISTRY                                 │ │
│  │                                                                    │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐            │ │
│  │  │ Integrations │  │  Transports  │  │   Handlers   │   ...     │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘            │ │
│  │                                                                    │ │
│  └────────────────────────────────┼──────────────────────────────────┘ │
│                                   │                                     │
│  ┌────────────────────────────────┼──────────────────────────────────┐ │
│  │                         CRA CORE                                   │ │
│  │                                                                    │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐         │ │
│  │  │   CARP   │  │  TRACE   │  │  Atlas   │  │  MCP     │         │ │
│  │  │ Resolver │  │Collector │  │  Loader  │  │  Server  │         │ │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘         │ │
│  │                                                                    │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                   │                                     │
│  ┌────────────────────────────────┼──────────────────────────────────┐ │
│  │                    WRAPPER TEMPLATES                               │ │
│  │                                                                    │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐               │ │
│  │  │ Claude Code │  │   OpenAI    │  │  LangChain  │   ...        │ │
│  │  │   Skill     │  │  Middleware │  │  Handler    │               │ │
│  │  └─────────────┘  └─────────────┘  └─────────────┘               │ │
│  │                                                                    │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 1. Plugin System

### Plugin Types

| Type | Purpose | Examples |
|------|---------|----------|
| **Integration** | Connect to external systems | LangChain, Slack, GitHub |
| **Transport** | Communication backends | WebSocket, gRPC, Queue |
| **Handler** | Custom checkpoint handlers | Sentiment analysis, validation |
| **Cache** | Cache backends | Redis, SQLite, S3 |
| **Storage** | TRACE storage backends | PostgreSQL, MongoDB, S3 |
| **Analytics** | Process TRACE data | Metrics, dashboards, alerts |

### Plugin Interface

```typescript
interface CRAPlugin {
  // Metadata
  name: string;
  version: string;
  description: string;
  author: string;

  // Lifecycle
  onLoad?(cra: CRACore): Promise<void>;
  onUnload?(): Promise<void>;

  // Extensions
  checkpointTypes?: CheckpointTypeDefinition[];
  handlers?: Record<string, CheckpointHandler>;
  transports?: Record<string, TransportBackend>;
  cacheBackends?: Record<string, CacheBackend>;
  storageBackends?: Record<string, StorageBackend>;

  // Configuration
  configSchema?: JSONSchema;
  defaultConfig?: Record<string, unknown>;
}
```

### Plugin Registry

```typescript
interface PluginRegistry {
  // Install from registry
  install(pluginName: string, version?: string): Promise<void>;

  // Install from URL/path
  installFrom(source: string): Promise<void>;

  // List installed
  list(): Plugin[];

  // Remove plugin
  uninstall(pluginName: string): Promise<void>;

  // Enable/disable
  enable(pluginName: string): void;
  disable(pluginName: string): void;
}
```

### Example Plugins

#### cra-plugin-langchain

```typescript
const langchainPlugin: CRAPlugin = {
  name: 'cra-plugin-langchain',
  version: '1.0.0',
  description: 'LangChain integration for CRA',

  onLoad: async (cra) => {
    // Register LangChain callback handler
    cra.registerIntegration('langchain', LangChainIntegration);
  },

  handlers: {
    'langchain_chain_start': async (ctx) => {
      // Inject context at chain start
      const contexts = await ctx.client.requestContext(ctx.input);
      return { injectContext: contexts, continue: true };
    },
  },
};
```

#### cra-plugin-slack

```typescript
const slackPlugin: CRAPlugin = {
  name: 'cra-plugin-slack',
  version: '1.0.0',
  description: 'Slack notifications for CRA events',

  configSchema: {
    type: 'object',
    properties: {
      webhookUrl: { type: 'string' },
      events: { type: 'array', items: { type: 'string' } },
    },
  },

  onLoad: async (cra) => {
    cra.on('trace_event', async (event) => {
      if (config.events.includes(event.type)) {
        await sendSlackNotification(event);
      }
    });
  },
};
```

---

## 2. Extensions

Extensions add capabilities beyond plugins—new protocols, UIs, or major features.

### Extension Types

| Type | Purpose | Examples |
|------|---------|----------|
| **Protocol** | New communication protocols | GraphQL API, SSE |
| **UI** | User interfaces | Dashboard, admin panel |
| **Analytics** | Advanced analytics | ML insights, anomaly detection |
| **Security** | Security features | SSO, encryption, audit |

### Extension Interface

```typescript
interface CRAExtension {
  name: string;
  version: string;
  type: ExtensionType;

  // May include own HTTP routes, workers, etc.
  routes?: Route[];
  workers?: Worker[];
  migrations?: Migration[];

  install(cra: CRACore): Promise<void>;
  uninstall(): Promise<void>;
}
```

### Example Extensions

#### cra-extension-dashboard

Web dashboard for monitoring CRA:

```typescript
const dashboardExtension: CRAExtension = {
  name: 'cra-extension-dashboard',
  version: '1.0.0',
  type: 'ui',

  routes: [
    { path: '/dashboard', handler: dashboardHandler },
    { path: '/dashboard/sessions', handler: sessionsHandler },
    { path: '/dashboard/atlases', handler: atlasesHandler },
    { path: '/api/metrics', handler: metricsAPI },
  ],

  install: async (cra) => {
    // Set up dashboard routes
    cra.addRoutes(this.routes);
  },
};
```

#### cra-extension-graphql

GraphQL API for CRA:

```typescript
const graphqlExtension: CRAExtension = {
  name: 'cra-extension-graphql',
  version: '1.0.0',
  type: 'protocol',

  routes: [
    { path: '/graphql', handler: graphqlHandler },
  ],

  install: async (cra) => {
    // Build schema from CRA types
    const schema = buildCRASchema(cra);
    this.graphqlHandler = createGraphQLHandler(schema);
  },
};
```

---

## 3. Atlas Marketplace

### Vision

A platform where stewards publish atlases and users discover/subscribe.

### Marketplace Features

| Feature | Description |
|---------|-------------|
| **Browse** | Search/filter atlases by domain, rating, price |
| **Preview** | See atlas contents, sample contexts |
| **Install** | One-click install to CRA |
| **Subscribe** | Subscription management for paid atlases |
| **Rate/Review** | Community ratings and reviews |
| **Analytics** | Usage stats for stewards |

### Marketplace Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                     ATLAS MARKETPLACE                                │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    WEB FRONTEND                              │   │
│  │  Browse | Search | Preview | Install | Manage                │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              │                                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    MARKETPLACE API                           │   │
│  │                                                               │   │
│  │  /atlases        - List, search, filter                      │   │
│  │  /atlases/:id    - Get atlas details                         │   │
│  │  /atlases/:id/install - Install atlas                        │   │
│  │  /subscriptions  - Manage subscriptions                      │   │
│  │  /stewards       - Steward profiles                          │   │
│  │  /reviews        - Ratings and reviews                       │   │
│  │                                                               │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              │                                       │
│  ┌──────────────┬────────────┼────────────┬──────────────┐         │
│  │   Atlas      │   User     │  Payment   │   Analytics  │         │
│  │   Storage    │   Service  │  Service   │   Service    │         │
│  └──────────────┴────────────┴────────────┴──────────────┘         │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Atlas Listing

```typescript
interface AtlasListing {
  id: string;
  name: string;
  description: string;
  version: string;

  steward: {
    id: string;
    name: string;
    verified: boolean;
  };

  pricing: {
    type: 'free' | 'subscription' | 'one_time';
    price?: number;
    currency?: string;
  };

  stats: {
    installs: number;
    rating: number;
    reviews: number;
  };

  domains: string[];
  tags: string[];
  preview: AtlasPreview;
}
```

### Steward Dashboard

Stewards manage their atlases:

```
┌─────────────────────────────────────────────────────────────────────┐
│                    STEWARD DASHBOARD                                 │
│                                                                      │
│  My Atlases                                                          │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │ VIB3+ Development     v6.0.0    ★★★★★ (47)    1,234 installs  │  │
│  │ Security Review       v2.1.0    ★★★★☆ (23)      567 installs  │  │
│  │ [+ Create New Atlas]                                           │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                      │
│  Analytics                                                           │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │ Total Installs: 1,801                                          │  │
│  │ Active Users: 342                                               │  │
│  │ Context Requests: 12,456 this month                            │  │
│  │ Positive Feedback: 89%                                          │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                      │
│  Feedback                                                            │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │ "Geometry formula was exactly what I needed" - helpful         │  │
│  │ "Needed more examples for audio reactivity" - not helpful      │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 4. Claude Code Skills

CRA can be packaged as a Claude Code skill for easy installation.

### Skill Manifest

```json
{
  "name": "cra-governance",
  "version": "1.0.0",
  "description": "CRA context, governance, and audit for Claude Code",

  "type": "mcp",

  "server": {
    "command": "npx",
    "args": ["-y", "cra-mcp-server"],
    "env": {
      "CRA_ATLASES_PATH": ".cra/atlases"
    }
  },

  "tools": [
    {
      "name": "cra_bootstrap",
      "description": "Initialize CRA governance for this session"
    },
    {
      "name": "cra_request_context",
      "description": "Get relevant context for current task"
    },
    {
      "name": "cra_report_action",
      "description": "Report action to audit trail"
    },
    {
      "name": "cra_feedback",
      "description": "Report if context was helpful"
    }
  ],

  "hooks": {
    "session_start": ".cra/hooks/session-start.js"
  },

  "setup": {
    "post_install": [
      "mkdir -p .cra/atlases",
      "mkdir -p .cra/trace"
    ]
  }
}
```

### Skill Installation

```bash
# Install CRA skill
claude skill install cra-governance

# Install with specific atlases
claude skill install cra-governance --atlas vib3-development

# From marketplace
claude skill install cra-governance --marketplace
```

### Skill Configuration

```json
// .claude/skills/cra-governance/config.json
{
  "atlases": [
    "vib3-development",
    "security-review"
  ],
  "checkpoints": {
    "keyword_match": true,
    "action_pre": true
  },
  "trace": {
    "mode": "async",
    "storage": "local"
  }
}
```

---

## 5. Atlas Creator Platform

A platform for creating, testing, and publishing atlases.

### Creator Studio

Web-based atlas editor:

```
┌─────────────────────────────────────────────────────────────────────┐
│                    ATLAS CREATOR STUDIO                              │
│                                                                      │
│  ┌─────────────┬───────────────────────────────────────────────┐   │
│  │             │                                                 │   │
│  │  STRUCTURE  │  EDITOR                                        │   │
│  │             │                                                 │   │
│  │  ▼ Atlas    │  Context Block: vib3-geometry-system           │   │
│  │    Metadata │  ┌─────────────────────────────────────────┐   │   │
│  │  ▼ Contexts │  │ # Geometry System                       │   │   │
│  │    ├─ vib3- │  │                                         │   │   │
│  │    │  geom  │  │ VIB3+ uses 24 geometries calculated     │   │   │
│  │    ├─ vib3- │  │ as: coreIndex * 8 + baseIndex          │   │   │
│  │    │  api   │  │                                         │   │   │
│  │    └─ ...   │  │ ## Base Geometries (0-7)                │   │   │
│  │  ▼ Policies │  │ 0. Torus                                │   │   │
│  │  ▼ Actions  │  │ 1. Sphere                               │   │   │
│  │             │  │ ...                                     │   │   │
│  │             │  └─────────────────────────────────────────┘   │   │
│  │             │                                                 │   │
│  │             │  SETTINGS                                       │   │
│  │             │  ┌─────────────────────────────────────────┐   │   │
│  │             │  │ Inject Mode: [on_match ▼]               │   │   │
│  │             │  │ Keywords: geometry, index, formula      │   │   │
│  │             │  │ Priority: 380                           │   │   │
│  │             │  └─────────────────────────────────────────┘   │   │
│  │             │                                                 │   │
│  └─────────────┴───────────────────────────────────────────────┘   │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  PREVIEW  │  TEST  │  VALIDATE  │  PUBLISH                   │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Creator Features

| Feature | Description |
|---------|-------------|
| **Visual Editor** | Markdown editor with preview |
| **Schema Validation** | Real-time validation |
| **Test Sandbox** | Test atlas with mock agent |
| **Version Control** | Git-like versioning |
| **Collaboration** | Team editing |
| **Analytics Preview** | Predicted keyword matches |
| **Publishing** | One-click publish to marketplace |

### Creator API

For programmatic atlas creation:

```typescript
interface CreatorAPI {
  // Atlas CRUD
  createAtlas(manifest: AtlasManifest): Promise<Atlas>;
  updateAtlas(id: string, manifest: AtlasManifest): Promise<Atlas>;
  deleteAtlas(id: string): Promise<void>;
  getAtlas(id: string): Promise<Atlas>;
  listAtlases(): Promise<Atlas[]>;

  // Version management
  createVersion(id: string, version: string): Promise<AtlasVersion>;
  listVersions(id: string): Promise<AtlasVersion[]>;
  rollback(id: string, version: string): Promise<void>;

  // Publishing
  publish(id: string, options: PublishOptions): Promise<PublishResult>;
  unpublish(id: string): Promise<void>;

  // Testing
  testAtlas(id: string, testCases: TestCase[]): Promise<TestResult[]>;
  validateAtlas(id: string): Promise<ValidationResult>;
}
```

### Atlas Templates

Pre-built templates for common use cases:

```
TEMPLATES
├── api-documentation/     # Template for API docs atlas
├── coding-standards/      # Template for code style atlas
├── security-policies/     # Template for security rules
├── onboarding/           # Template for onboarding context
└── domain-knowledge/     # Generic domain knowledge template
```

---

## 6. Integration Ecosystem

### First-Party Integrations

| Integration | Type | Purpose |
|-------------|------|---------|
| **Claude Code** | Skill | Native Claude Code integration |
| **OpenAI** | Middleware | OpenAI API wrapper |
| **LangChain** | Handler | LangChain callback handler |
| **LlamaIndex** | Handler | LlamaIndex callback |
| **AutoGen** | Plugin | Microsoft AutoGen integration |

### Third-Party Integration Points

```typescript
interface IntegrationPoints {
  // Wrap any LLM client
  wrapClient<T>(client: T, options: WrapOptions): T;

  // Add to any callback system
  createCallbackHandler(options: CallbackOptions): CallbackHandler;

  // HTTP middleware
  createMiddleware(options: MiddlewareOptions): Middleware;

  // CLI wrapper
  createCLIWrapper(options: CLIOptions): CLIWrapper;
}
```

### Webhook Integrations

CRA can send events to external systems:

```json
{
  "integrations": {
    "webhooks": [
      {
        "url": "https://api.example.com/cra-events",
        "events": ["session_started", "policy_violation"],
        "headers": {
          "Authorization": "Bearer ${CRA_WEBHOOK_TOKEN}"
        }
      }
    ],
    "slack": {
      "webhook_url": "https://hooks.slack.com/...",
      "events": ["policy_violation", "error"]
    },
    "datadog": {
      "api_key": "${DATADOG_API_KEY}",
      "metrics": ["session_count", "context_requests"]
    }
  }
}
```

---

## 7. Storage & Hosting

### Atlas Storage Options

| Option | Best For | Features |
|--------|----------|----------|
| **Local** | Development | File system, git-friendly |
| **S3/GCS** | Production | Scalable, versioned |
| **CRA Cloud** | Managed | Hosted, marketplace integrated |

### TRACE Storage Options

| Option | Best For | Features |
|--------|----------|----------|
| **Local** | Development | SQLite, file-based |
| **PostgreSQL** | Production | Relational, queryable |
| **MongoDB** | Large scale | Document store, flexible |
| **S3** | Archive | Cheap, immutable |

### CRA Cloud (Future)

Managed CRA hosting:

```
┌─────────────────────────────────────────────────────────────────────┐
│                        CRA CLOUD                                     │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    MANAGED CRA                               │   │
│  │  • Hosted CRA Core                                           │   │
│  │  • Managed atlas storage                                     │   │
│  │  • TRACE storage & analytics                                 │   │
│  │  • Automatic scaling                                         │   │
│  │  • Monitoring & alerts                                       │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    MARKETPLACE                               │   │
│  │  • Atlas discovery                                           │   │
│  │  • Subscriptions                                             │   │
│  │  • Steward payouts                                           │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 8. Roadmap

### Now

- [x] Core CRA (Rust)
- [x] Atlas format
- [x] Basic TRACE
- [ ] Wrapper protocol
- [ ] Checkpoint system

### Next

- [ ] MCP server
- [ ] Claude Code skill
- [ ] Plugin system
- [ ] First-party integrations

### Later

- [ ] Marketplace MVP
- [ ] Creator studio
- [ ] Dashboard extension
- [ ] Analytics extension

### Future

- [ ] CRA Cloud
- [ ] Enterprise features
- [ ] Advanced analytics
- [ ] Multi-agent coordination

---

## Summary

The CRA ecosystem is designed to grow:

| Layer | Components |
|-------|------------|
| **Core** | Rust library, protocols, MCP server |
| **Plugins** | Integrations, transports, handlers |
| **Extensions** | Dashboards, APIs, security |
| **Marketplace** | Atlas discovery, subscriptions |
| **Creator** | Studio, templates, testing |
| **Cloud** | Managed hosting, enterprise |

All built on the principle: **start minimal, extend as needed**.
