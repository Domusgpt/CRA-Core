# TypeScript SDK Implementation Plan

Complementary TypeScript components to extend the Python CRA core.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         CRA Ecosystem                                    │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────────────────────────┐  ┌──────────────────────────────┐  │
│  │      Python Core (cra-core)     │  │   TypeScript (cra-js)        │  │
│  │                                 │  │                              │  │
│  │  ┌───────────┐ ┌─────────────┐  │  │  ┌────────────────────────┐  │  │
│  │  │  Runtime  │ │   FastAPI   │◄─┼──┼──│     Client SDK         │  │  │
│  │  │  Server   │ │   /v1/*     │  │  │  │  (HTTP + WebSocket)    │  │  │
│  │  └───────────┘ └─────────────┘  │  │  └────────────────────────┘  │  │
│  │                                 │  │                              │  │
│  │  ┌───────────┐ ┌─────────────┐  │  │  ┌────────────────────────┐  │  │
│  │  │   CARP    │ │   TRACE     │  │  │  │     MCP Server         │  │  │
│  │  │  Engine   │ │  Collector  │  │  │  │  (Claude Desktop)      │  │  │
│  │  └───────────┘ └─────────────┘  │  │  └────────────────────────┘  │  │
│  │                                 │  │                              │  │
│  │  ┌───────────┐ ┌─────────────┐  │  │  ┌────────────────────────┐  │  │
│  │  │  Storage  │ │    Auth     │  │  │  │   VS Code Extension    │  │  │
│  │  │ Postgres  │ │  JWT/RBAC   │  │  │  │  (Atlas authoring)     │  │  │
│  │  └───────────┘ └─────────────┘  │  │  └────────────────────────┘  │  │
│  │                                 │  │                              │  │
│  │  ┌───────────┐ ┌─────────────┐  │  │  ┌────────────────────────┐  │  │
│  │  │   CLI     │ │ Middleware  │  │  │  │    Web Dashboard       │  │  │
│  │  │  (typer)  │ │ Lang/Crew   │  │  │  │  (React + Traces)      │  │  │
│  │  └───────────┘ └─────────────┘  │  │  └────────────────────────┘  │  │
│  │                                 │  │                              │  │
│  └─────────────────────────────────┘  └──────────────────────────────┘  │
│                                                                          │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                    Shared Protocol Specs                          │  │
│  │        CARP/1.0 Schema  •  TRACE/1.0 Schema  •  Atlas Format      │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Package Structure

```
cra-js/
├── packages/
│   ├── types/              # Shared TypeScript types (CARP, TRACE, Atlas)
│   ├── client/             # HTTP/WebSocket client for Python runtime
│   ├── mcp-server/         # MCP server exposing CRA to Claude Desktop
│   ├── vscode/             # VS Code extension for Atlas authoring
│   ├── dashboard/          # React web dashboard for traces/monitoring
│   └── edge/               # Edge runtime adapter (Cloudflare/Vercel)
├── package.json            # Workspace root
├── tsconfig.base.json      # Shared TypeScript config
└── turbo.json              # Turborepo build config
```

---

## Phase 1: Foundation (Types + Client SDK)

### 1.1 `@cra/types` — Shared Type Definitions

```typescript
// packages/types/src/carp.ts
export interface CARPRequest {
  carp_version: '1.0';
  request_id: string;
  timestamp: string;
  operation: 'resolve' | 'execute' | 'validate';
  requester: {
    agent_id: string;
    session_id: string;
    parent_session_id?: string;
  };
  task: {
    goal: string;
    risk_tier?: 'low' | 'medium' | 'high' | 'critical';
    context_hints?: string[];
    required_capabilities?: string[];
  };
  atlas_ids?: string[];
  context?: Record<string, unknown>;
}

export interface CARPResolution {
  resolution_id: string;
  request_id: string;
  timestamp: string;
  decision: {
    type: 'allow' | 'deny' | 'requires_approval' | 'partial';
    reason?: string;
    approval_id?: string;
  };
  context_blocks: ContextBlock[];
  allowed_actions: ActionDefinition[];
  denied_actions: DeniedAction[];
  constraints: Constraint[];
  ttl_seconds: number;
  trace_id: string;
}

// packages/types/src/trace.ts
export interface TRACEEvent {
  trace_version: '1.0';
  event_id: string;
  trace_id: string;
  span_id: string;
  parent_span_id?: string;
  session_id: string;
  sequence: number;
  timestamp: string;
  event_type: TRACEEventType;
  payload: Record<string, unknown>;
  event_hash: string;
  previous_event_hash: string;
}

export type TRACEEventType =
  | 'session.started'
  | 'session.ended'
  | 'carp.request.received'
  | 'carp.resolution.completed'
  | 'action.requested'
  | 'action.approved'
  | 'action.denied'
  | 'action.executed'
  | 'action.failed'
  | 'context.injected'
  | 'policy.evaluated'
  | 'policy.violated';

// packages/types/src/atlas.ts
export interface AtlasManifest {
  atlas_id: string;
  version: string;
  name: string;
  description: string;
  domains: string[];
  capabilities: Capability[];
  policies: Policy[];
  actions: ActionDefinition[];
  context_packs: ContextPack[];
  adapters?: Record<string, AdapterConfig>;
}
```

**Deliverables:**
- Complete TypeScript types matching Python Pydantic models
- JSON Schema generation for validation
- Zod schemas for runtime validation
- Published as `@cra/types` on npm

---

### 1.2 `@cra/client` — TypeScript Client SDK

```typescript
// packages/client/src/client.ts
import type { CARPRequest, CARPResolution, TRACEEvent } from '@cra/types';

export interface CRAClientConfig {
  baseUrl: string;
  apiKey?: string;
  jwtToken?: string;
  timeout?: number;
  retries?: number;
}

export class CRAClient {
  private config: CRAClientConfig;
  private ws?: WebSocket;

  constructor(config: CRAClientConfig) {
    this.config = config;
  }

  // Session Management
  async createSession(params: {
    agentId: string;
    goal: string;
    atlasId?: string;
  }): Promise<Session> { }

  async getSession(sessionId: string): Promise<Session> { }

  async endSession(sessionId: string): Promise<void> { }

  // CARP Operations
  async resolve(request: Omit<CARPRequest, 'carp_version' | 'request_id' | 'timestamp'>): Promise<CARPResolution> { }

  async execute(params: {
    sessionId: string;
    actionId: string;
    parameters: Record<string, unknown>;
  }): Promise<ExecutionResult> { }

  // Atlas Management
  async listAtlases(): Promise<AtlasSummary[]> { }

  async getAtlas(atlasId: string): Promise<AtlasManifest> { }

  async loadAtlas(manifest: AtlasManifest): Promise<void> { }

  // Trace Streaming
  subscribeToTraces(
    sessionId: string,
    callback: (event: TRACEEvent) => void
  ): () => void { }

  async getTrace(sessionId: string): Promise<TRACEEvent[]> { }

  // Health
  async health(): Promise<HealthStatus> { }
}

// Convenience functions
export function createClient(config: CRAClientConfig): CRAClient {
  return new CRAClient(config);
}

// React hook (optional)
export function useCRA(config: CRAClientConfig) {
  const client = useMemo(() => createClient(config), [config]);
  // ... React Query integration
}
```

**Usage Example:**

```typescript
import { createClient } from '@cra/client';

const cra = createClient({
  baseUrl: 'http://localhost:8420',
  apiKey: 'cra_xxx',
});

// Start session
const session = await cra.createSession({
  agentId: 'my-agent',
  goal: 'Help with customer support',
  atlasId: 'com.example.customer-support',
});

// Resolve capabilities
const resolution = await cra.resolve({
  operation: 'resolve',
  requester: {
    agent_id: session.agentId,
    session_id: session.sessionId,
  },
  task: {
    goal: 'Look up ticket #12345',
  },
});

console.log('Allowed actions:', resolution.allowed_actions);

// Execute action
const result = await cra.execute({
  sessionId: session.sessionId,
  actionId: 'ticket.lookup',
  parameters: { ticket_id: '12345' },
});

// Stream traces
const unsubscribe = cra.subscribeToTraces(session.sessionId, (event) => {
  console.log('Trace event:', event.event_type);
});

// Cleanup
await cra.endSession(session.sessionId);
unsubscribe();
```

**Deliverables:**
- Full HTTP client with retry logic
- WebSocket support for trace streaming
- React Query hooks (optional)
- Published as `@cra/client` on npm

---

## Phase 2: MCP Server

### 2.1 `@cra/mcp-server` — Model Context Protocol Server

```typescript
// packages/mcp-server/src/server.ts
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { CRAClient } from '@cra/client';

export interface MCPServerConfig {
  craBaseUrl: string;
  craApiKey?: string;
  atlasIds?: string[];
  serverName?: string;
  serverVersion?: string;
}

export async function createMCPServer(config: MCPServerConfig) {
  const cra = new CRAClient({
    baseUrl: config.craBaseUrl,
    apiKey: config.craApiKey,
  });

  const server = new Server(
    {
      name: config.serverName ?? 'cra-mcp-server',
      version: config.serverVersion ?? '1.0.0',
    },
    {
      capabilities: {
        tools: {},
        resources: {},
        prompts: {},
      },
    }
  );

  // Dynamic tool registration from CRA resolution
  server.setRequestHandler('tools/list', async () => {
    const atlases = await cra.listAtlases();
    const tools = [];

    for (const atlas of atlases) {
      const full = await cra.getAtlas(atlas.atlas_id);
      for (const action of full.actions) {
        tools.push({
          name: `cra_${action.action_id.replace(/\./g, '_')}`,
          description: action.description,
          inputSchema: action.parameters_schema ?? { type: 'object', properties: {} },
        });
      }
    }

    return { tools };
  });

  // Tool execution through CRA
  server.setRequestHandler('tools/call', async (request) => {
    const { name, arguments: args } = request.params;

    // Extract action ID from tool name
    const actionId = name.replace('cra_', '').replace(/_/g, '.');

    // Create session if needed
    const session = await cra.createSession({
      agentId: 'mcp-client',
      goal: `Execute ${actionId}`,
    });

    try {
      const result = await cra.execute({
        sessionId: session.sessionId,
        actionId,
        parameters: args,
      });

      return {
        content: [
          {
            type: 'text',
            text: JSON.stringify(result, null, 2),
          },
        ],
        isError: result.status === 'error',
      };
    } finally {
      await cra.endSession(session.sessionId);
    }
  });

  // Resources: Expose Atlas context packs
  server.setRequestHandler('resources/list', async () => {
    const atlases = await cra.listAtlases();
    const resources = [];

    for (const atlas of atlases) {
      resources.push({
        uri: `cra://atlas/${atlas.atlas_id}`,
        name: atlas.name,
        description: atlas.description,
        mimeType: 'application/json',
      });
    }

    return { resources };
  });

  server.setRequestHandler('resources/read', async (request) => {
    const { uri } = request.params;
    const atlasId = uri.replace('cra://atlas/', '');
    const atlas = await cra.getAtlas(atlasId);

    return {
      contents: [
        {
          uri,
          mimeType: 'application/json',
          text: JSON.stringify(atlas, null, 2),
        },
      ],
    };
  });

  // Prompts: Pre-built agent prompts
  server.setRequestHandler('prompts/list', async () => {
    return {
      prompts: [
        {
          name: 'cra_governed_agent',
          description: 'System prompt for a CRA-governed agent',
          arguments: [
            { name: 'atlas_id', description: 'Atlas to use', required: true },
            { name: 'goal', description: 'Agent goal', required: true },
          ],
        },
      ],
    };
  });

  server.setRequestHandler('prompts/get', async (request) => {
    const { name, arguments: args } = request.params;

    if (name === 'cra_governed_agent') {
      const session = await cra.createSession({
        agentId: 'prompt-generator',
        goal: args.goal,
        atlasId: args.atlas_id,
      });

      const resolution = await cra.resolve({
        operation: 'resolve',
        requester: {
          agent_id: 'prompt-generator',
          session_id: session.sessionId,
        },
        task: { goal: args.goal },
        atlas_ids: [args.atlas_id],
      });

      // Build system prompt from resolution
      const contextText = resolution.context_blocks
        .map((b) => b.content)
        .join('\n\n');

      const actionsText = resolution.allowed_actions
        .map((a) => `- ${a.action_id}: ${a.description}`)
        .join('\n');

      return {
        messages: [
          {
            role: 'user',
            content: {
              type: 'text',
              text: `You are a CRA-governed agent with the following context:\n\n${contextText}\n\nAvailable actions:\n${actionsText}\n\nGoal: ${args.goal}`,
            },
          },
        ],
      };
    }

    throw new Error(`Unknown prompt: ${name}`);
  });

  return server;
}

// CLI entry point
async function main() {
  const server = await createMCPServer({
    craBaseUrl: process.env.CRA_URL ?? 'http://localhost:8420',
    craApiKey: process.env.CRA_API_KEY,
  });

  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch(console.error);
```

**Claude Desktop Configuration:**

```json
{
  "mcpServers": {
    "cra": {
      "command": "npx",
      "args": ["@cra/mcp-server"],
      "env": {
        "CRA_URL": "http://localhost:8420",
        "CRA_API_KEY": "your-api-key"
      }
    }
  }
}
```

**Deliverables:**
- Full MCP server implementation
- Dynamic tool registration from CRA
- Resource exposure for Atlases
- Prompt templates for governed agents
- npm package with CLI entry point
- Claude Desktop configuration docs

---

## Phase 3: VS Code Extension

### 3.1 `@cra/vscode` — Atlas Authoring Extension

**Features:**

| Feature | Description |
|---------|-------------|
| **Atlas Validation** | Real-time validation of atlas.yaml/json files |
| **IntelliSense** | Autocomplete for action IDs, capabilities, policies |
| **Schema Hover** | Documentation on hover for all Atlas fields |
| **Policy Linting** | Detect policy conflicts and coverage gaps |
| **Trace Viewer** | Side panel showing live trace events |
| **Action Testing** | Execute actions directly from the editor |
| **Diff View** | Compare trace diffs for regression testing |

**Extension Structure:**

```
packages/vscode/
├── src/
│   ├── extension.ts           # Extension entry point
│   ├── providers/
│   │   ├── completion.ts      # IntelliSense provider
│   │   ├── diagnostics.ts     # Validation/linting
│   │   ├── hover.ts           # Hover documentation
│   │   └── codelens.ts        # Inline action runners
│   ├── views/
│   │   ├── tracePanel.ts      # Trace viewer webview
│   │   ├── atlasExplorer.ts   # Atlas tree view
│   │   └── actionRunner.ts    # Action execution panel
│   ├── commands/
│   │   ├── validate.ts        # Validate Atlas command
│   │   ├── execute.ts         # Execute action command
│   │   └── diff.ts            # Trace diff command
│   └── client.ts              # CRA client integration
├── syntaxes/
│   └── atlas.tmLanguage.json  # Syntax highlighting
├── schemas/
│   └── atlas.schema.json      # JSON Schema for validation
├── package.json               # Extension manifest
└── tsconfig.json
```

**Key Implementation:**

```typescript
// packages/vscode/src/extension.ts
import * as vscode from 'vscode';
import { CRAClient } from '@cra/client';
import { AtlasCompletionProvider } from './providers/completion';
import { AtlasDiagnosticsProvider } from './providers/diagnostics';
import { TracePanel } from './views/tracePanel';

export async function activate(context: vscode.ExtensionContext) {
  // Initialize CRA client
  const config = vscode.workspace.getConfiguration('cra');
  const client = new CRAClient({
    baseUrl: config.get('serverUrl') ?? 'http://localhost:8420',
    apiKey: config.get('apiKey'),
  });

  // Register providers
  context.subscriptions.push(
    vscode.languages.registerCompletionItemProvider(
      { scheme: 'file', pattern: '**/atlas.{json,yaml,yml}' },
      new AtlasCompletionProvider(client)
    ),
    vscode.languages.registerHoverProvider(
      { scheme: 'file', pattern: '**/atlas.{json,yaml,yml}' },
      new AtlasHoverProvider()
    )
  );

  // Register diagnostics
  const diagnostics = vscode.languages.createDiagnosticCollection('cra');
  context.subscriptions.push(diagnostics);

  const diagnosticsProvider = new AtlasDiagnosticsProvider(client, diagnostics);
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument((doc) => {
      if (doc.fileName.includes('atlas.')) {
        diagnosticsProvider.validate(doc);
      }
    })
  );

  // Register commands
  context.subscriptions.push(
    vscode.commands.registerCommand('cra.validateAtlas', async () => {
      const editor = vscode.window.activeTextEditor;
      if (editor) {
        await diagnosticsProvider.validate(editor.document);
        vscode.window.showInformationMessage('Atlas validated');
      }
    }),
    vscode.commands.registerCommand('cra.executeAction', async (actionId: string) => {
      const panel = new ActionRunnerPanel(client);
      await panel.show(actionId);
    }),
    vscode.commands.registerCommand('cra.showTraces', async () => {
      const panel = new TracePanel(context.extensionUri, client);
      panel.show();
    })
  );

  // Register tree view
  const atlasExplorer = new AtlasExplorerProvider(client);
  vscode.window.registerTreeDataProvider('craAtlases', atlasExplorer);

  // Status bar
  const statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right);
  statusBar.text = '$(shield) CRA';
  statusBar.command = 'cra.showTraces';
  statusBar.show();
  context.subscriptions.push(statusBar);

  // Check connection
  try {
    await client.health();
    statusBar.text = '$(shield) CRA Connected';
  } catch {
    statusBar.text = '$(shield) CRA Disconnected';
  }
}
```

**Trace Viewer Webview:**

```typescript
// packages/vscode/src/views/tracePanel.ts
export class TracePanel {
  private panel: vscode.WebviewPanel | undefined;
  private client: CRAClient;

  constructor(private extensionUri: vscode.Uri, client: CRAClient) {
    this.client = client;
  }

  show() {
    this.panel = vscode.window.createWebviewPanel(
      'craTraces',
      'CRA Traces',
      vscode.ViewColumn.Two,
      { enableScripts: true }
    );

    this.panel.webview.html = this.getHtml();

    // Subscribe to traces
    this.client.subscribeToTraces('*', (event) => {
      this.panel?.webview.postMessage({ type: 'trace', event });
    });
  }

  private getHtml() {
    return `
      <!DOCTYPE html>
      <html>
      <head>
        <style>
          .event { padding: 8px; border-bottom: 1px solid #333; }
          .event-type { font-weight: bold; color: #569cd6; }
          .timestamp { color: #888; font-size: 12px; }
          .payload { font-family: monospace; margin-top: 4px; }
        </style>
      </head>
      <body>
        <div id="events"></div>
        <script>
          const container = document.getElementById('events');
          window.addEventListener('message', (e) => {
            if (e.data.type === 'trace') {
              const event = e.data.event;
              const div = document.createElement('div');
              div.className = 'event';
              div.innerHTML = \`
                <span class="event-type">\${event.event_type}</span>
                <span class="timestamp">\${event.timestamp}</span>
                <pre class="payload">\${JSON.stringify(event.payload, null, 2)}</pre>
              \`;
              container.prepend(div);
            }
          });
        </script>
      </body>
      </html>
    `;
  }
}
```

**Deliverables:**
- Full VS Code extension
- Atlas validation and IntelliSense
- Live trace viewer
- Action execution from editor
- Published on VS Code Marketplace

---

## Phase 4: Web Dashboard

### 4.1 `@cra/dashboard` — React Monitoring Dashboard

**Features:**

| Feature | Description |
|---------|-------------|
| **Session Monitor** | Live view of active sessions |
| **Trace Explorer** | Search and filter trace events |
| **Trace Timeline** | Visual timeline of session events |
| **Policy Violations** | Alert dashboard for violations |
| **Atlas Browser** | Browse and inspect loaded Atlases |
| **Metrics Dashboard** | Charts for latency, throughput, errors |
| **Replay Player** | Step through traces with state reconstruction |

**Tech Stack:**
- React 18 + TypeScript
- TanStack Query for data fetching
- Tailwind CSS for styling
- Recharts for visualizations
- WebSocket for real-time updates

**Component Structure:**

```
packages/dashboard/
├── src/
│   ├── app/
│   │   ├── layout.tsx
│   │   ├── page.tsx                 # Dashboard home
│   │   ├── sessions/
│   │   │   ├── page.tsx             # Session list
│   │   │   └── [id]/page.tsx        # Session detail
│   │   ├── traces/
│   │   │   ├── page.tsx             # Trace explorer
│   │   │   └── [id]/page.tsx        # Trace detail
│   │   ├── atlases/
│   │   │   ├── page.tsx             # Atlas browser
│   │   │   └── [id]/page.tsx        # Atlas detail
│   │   └── violations/
│   │       └── page.tsx             # Policy violations
│   ├── components/
│   │   ├── TraceTimeline.tsx        # Visual timeline
│   │   ├── TraceEvent.tsx           # Single event card
│   │   ├── SessionCard.tsx          # Session summary
│   │   ├── AtlasTree.tsx            # Atlas structure view
│   │   ├── PolicyViolationAlert.tsx # Violation alert
│   │   ├── MetricsChart.tsx         # Metrics visualization
│   │   └── ReplayPlayer.tsx         # Trace replay controls
│   ├── hooks/
│   │   ├── useSessions.ts           # Session data hook
│   │   ├── useTraces.ts             # Trace data hook
│   │   ├── useWebSocket.ts          # WebSocket connection
│   │   └── useCRA.ts                # CRA client hook
│   └── lib/
│       ├── client.ts                # CRA client instance
│       └── utils.ts                 # Utilities
├── package.json
└── tailwind.config.js
```

**Key Components:**

```tsx
// packages/dashboard/src/components/TraceTimeline.tsx
import { TRACEEvent } from '@cra/types';

interface TraceTimelineProps {
  events: TRACEEvent[];
  onEventClick: (event: TRACEEvent) => void;
}

export function TraceTimeline({ events, onEventClick }: TraceTimelineProps) {
  const grouped = groupBySpan(events);

  return (
    <div className="relative">
      {/* Time axis */}
      <div className="absolute left-0 top-0 bottom-0 w-px bg-gray-700" />

      {/* Events */}
      {events.map((event, i) => (
        <div
          key={event.event_id}
          className="relative pl-8 py-2 hover:bg-gray-800 cursor-pointer"
          onClick={() => onEventClick(event)}
        >
          {/* Connector */}
          <div className="absolute left-0 top-1/2 w-8 h-px bg-gray-700" />
          <div className="absolute left-0 top-1/2 w-3 h-3 -mt-1.5 rounded-full bg-blue-500" />

          {/* Event content */}
          <div className="flex items-center gap-4">
            <span className={`font-mono text-sm ${getEventColor(event.event_type)}`}>
              {event.event_type}
            </span>
            <span className="text-gray-500 text-xs">
              {formatTimestamp(event.timestamp)}
            </span>
          </div>

          {/* Payload preview */}
          <div className="mt-1 text-gray-400 text-sm truncate max-w-md">
            {JSON.stringify(event.payload)}
          </div>
        </div>
      ))}
    </div>
  );
}

function getEventColor(type: string): string {
  if (type.includes('error') || type.includes('denied') || type.includes('violated')) {
    return 'text-red-400';
  }
  if (type.includes('completed') || type.includes('approved')) {
    return 'text-green-400';
  }
  return 'text-blue-400';
}
```

```tsx
// packages/dashboard/src/components/ReplayPlayer.tsx
import { useState, useCallback } from 'react';
import { TRACEEvent } from '@cra/types';

interface ReplayPlayerProps {
  events: TRACEEvent[];
}

export function ReplayPlayer({ events }: ReplayPlayerProps) {
  const [currentIndex, setCurrentIndex] = useState(0);
  const [isPlaying, setIsPlaying] = useState(false);
  const [playbackSpeed, setPlaybackSpeed] = useState(1);

  const currentEvent = events[currentIndex];

  const play = useCallback(() => {
    setIsPlaying(true);
    // Advance through events based on actual timestamps
  }, []);

  const pause = () => setIsPlaying(false);
  const stepForward = () => setCurrentIndex((i) => Math.min(i + 1, events.length - 1));
  const stepBackward = () => setCurrentIndex((i) => Math.max(i - 1, 0));
  const reset = () => setCurrentIndex(0);

  return (
    <div className="bg-gray-900 rounded-lg p-4">
      {/* Progress bar */}
      <div className="relative h-2 bg-gray-700 rounded mb-4">
        <div
          className="absolute h-full bg-blue-500 rounded"
          style={{ width: `${(currentIndex / (events.length - 1)) * 100}%` }}
        />
      </div>

      {/* Controls */}
      <div className="flex items-center gap-4 justify-center">
        <button onClick={reset}>⏮</button>
        <button onClick={stepBackward}>⏪</button>
        <button onClick={isPlaying ? pause : play}>
          {isPlaying ? '⏸' : '▶'}
        </button>
        <button onClick={stepForward}>⏩</button>
        <select
          value={playbackSpeed}
          onChange={(e) => setPlaybackSpeed(Number(e.target.value))}
        >
          <option value={0.5}>0.5x</option>
          <option value={1}>1x</option>
          <option value={2}>2x</option>
          <option value={4}>4x</option>
        </select>
      </div>

      {/* Current event */}
      <div className="mt-4 p-4 bg-gray-800 rounded">
        <div className="text-blue-400 font-mono">{currentEvent?.event_type}</div>
        <pre className="mt-2 text-sm text-gray-300 overflow-auto max-h-64">
          {JSON.stringify(currentEvent?.payload, null, 2)}
        </pre>
      </div>

      {/* Event counter */}
      <div className="mt-2 text-center text-gray-500 text-sm">
        Event {currentIndex + 1} of {events.length}
      </div>
    </div>
  );
}
```

**Deliverables:**
- Full React dashboard application
- Real-time session monitoring
- Trace exploration and filtering
- Visual trace replay
- Policy violation alerts
- Deployable as static site or Docker container

---

## Phase 5: Edge Runtime

### 5.1 `@cra/edge` — Edge/Serverless Adapter

**Supported Platforms:**
- Cloudflare Workers
- Vercel Edge Functions
- Deno Deploy
- AWS Lambda@Edge

```typescript
// packages/edge/src/cloudflare.ts
import { CRAClient } from '@cra/client';

export interface CloudflareCRAConfig {
  craUrl: string;
  craApiKey: string;
  atlasIds?: string[];
}

export function createCRAHandler(config: CloudflareCRAConfig) {
  const client = new CRAClient({
    baseUrl: config.craUrl,
    apiKey: config.craApiKey,
  });

  return {
    async fetch(request: Request, env: any, ctx: ExecutionContext): Promise<Response> {
      const url = new URL(request.url);

      // Health check
      if (url.pathname === '/health') {
        const health = await client.health();
        return Response.json(health);
      }

      // Proxy to CRA with edge caching
      if (url.pathname.startsWith('/v1/')) {
        const cacheKey = `${url.pathname}:${await hashBody(request)}`;
        const cache = caches.default;

        // Check cache for resolutions
        if (request.method === 'POST' && url.pathname === '/v1/resolve') {
          const cached = await cache.match(cacheKey);
          if (cached) return cached;
        }

        // Forward to CRA
        const response = await fetch(`${config.craUrl}${url.pathname}`, {
          method: request.method,
          headers: {
            'Content-Type': 'application/json',
            'X-API-Key': config.craApiKey,
          },
          body: request.body,
        });

        // Cache successful resolutions
        if (response.ok && url.pathname === '/v1/resolve') {
          const cloned = response.clone();
          ctx.waitUntil(cache.put(cacheKey, cloned));
        }

        return response;
      }

      return new Response('Not Found', { status: 404 });
    },
  };
}

// Vercel Edge adapter
export function createVercelHandler(config: CloudflareCRAConfig) {
  const handler = createCRAHandler(config);

  return async (request: Request) => {
    return handler.fetch(request, {}, { waitUntil: () => {} });
  };
}
```

**Deliverables:**
- Cloudflare Workers adapter
- Vercel Edge adapter
- Deno Deploy adapter
- Edge caching for resolutions
- Geographic routing support

---

## Implementation Timeline

| Phase | Duration | Dependencies |
|-------|----------|--------------|
| Phase 1: Types + Client | 2 weeks | None |
| Phase 2: MCP Server | 2 weeks | Phase 1 |
| Phase 3: VS Code Extension | 3 weeks | Phase 1 |
| Phase 4: Web Dashboard | 4 weeks | Phase 1 |
| Phase 5: Edge Runtime | 2 weeks | Phase 1 |

**Total: ~13 weeks for full TypeScript ecosystem**

---

## Integration Points with Python Core

| TypeScript Component | Python Endpoint | Protocol |
|---------------------|-----------------|----------|
| Client SDK | `/v1/*` | HTTP REST |
| MCP Server | `/v1/*` | HTTP REST |
| VS Code Extension | `/v1/*` | HTTP REST + WebSocket |
| Dashboard | `/v1/*` + `/ws/traces` | HTTP REST + WebSocket |
| Edge Runtime | `/v1/*` | HTTP REST |

**Shared Contracts:**
- CARP/1.0 request/response schemas
- TRACE/1.0 event schemas
- Atlas manifest format
- Authentication (JWT/API Key)

---

## Success Metrics

| Metric | Target |
|--------|--------|
| npm weekly downloads | 1,000+ |
| VS Code extension installs | 500+ |
| MCP server adoption | 100+ Claude Desktop users |
| Dashboard deployments | 50+ |
| GitHub stars (cra-js) | 200+ |

---

## Next Steps

1. **Initialize cra-js repository** with Turborepo
2. **Port types from Python** Pydantic models to TypeScript
3. **Implement client SDK** with full test coverage
4. **Build MCP server** and test with Claude Desktop
5. **Develop VS Code extension** MVP
6. **Create dashboard** with core features
7. **Document integration** patterns

---

*This plan complements the Python CRA core, providing TypeScript tooling for the broader developer ecosystem.*
