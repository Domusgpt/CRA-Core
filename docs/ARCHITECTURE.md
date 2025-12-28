# CRA Architecture v0.2

## System Overview

CRA (Context Registry Agents) is a protocol-first authority layer for agentic systems. It provides governed, platform-aware context and permitted actions to LLM agents through two foundational protocols: CARP (Context & Action Resolution Protocol) and TRACE (Telemetry & Replay Artifact Contract Envelope).

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              CRA ARCHITECTURE                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐     CARP Request      ┌──────────────────────────────┐   │
│  │              │ ───────────────────── │                              │   │
│  │  LLM Agent   │                       │       CRA Runtime            │   │
│  │  (Requester) │ ◄─────────────────── │   (Resolver + Policy)        │   │
│  │              │     CARP Resolution   │                              │   │
│  └──────────────┘                       └──────────────┬───────────────┘   │
│         │                                              │                    │
│         │ Execute Action                               │ Load               │
│         ▼                                              ▼                    │
│  ┌──────────────┐                       ┌──────────────────────────────┐   │
│  │   Platform   │                       │         Atlas Store          │   │
│  │   Adapter    │                       │   (Context + Policies +      │   │
│  │ (OpenAI/     │                       │    Adapters + Tests)         │   │
│  │  Claude/MCP) │                       └──────────────────────────────┘   │
│  └──────────────┘                                                           │
│         │                                                                    │
│         │ TRACE Events        ┌──────────────────────────────────────────┐ │
│         ▼                     │              Storage Layer                │ │
│  ┌─────────────────┐          │   (File / PostgreSQL Persistence)        │ │
│  │ TRACE Collector │─────────►│                                          │ │
│  │ + Redaction     │          └──────────────────────────────────────────┘ │
│  └─────────────────┘                                                        │
│         │                                                                    │
│         ├─────────────────────────────────────────────────────────────────┐ │
│         ▼                                                                  │ │
│  ┌──────────────────────────────────────────────────────────────────────┐ │ │
│  │                        HTTP/WebSocket Server                          │ │ │
│  │         (REST API + SSE Streaming + WebSocket Trace)                 │ │ │
│  └──────────────────────────────────────────────────────────────────────┘ │ │
│         │                                                                  │ │
│         ▼                                                                  ▼ │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                        Dual-Mode UI                                   │   │
│  │    Human Terminal Interface    │    Agent JSON API                   │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Core Services / Modules

### 1. CRA Runtime (`packages/runtime`)

The authoritative resolver and policy engine. All context resolution and action permissions flow through here.

**Responsibilities:**
- CARP request processing
- Policy evaluation and enforcement
- Context block assembly with TTL management
- Action permission resolution
- Evidence attachment
- TRACE event emission

**Key Interfaces:**
```typescript
interface CRARuntime {
  resolve(request: CARPRequest): Promise<CARPResolution>;
  execute(action: CARPAction): Promise<CARPExecutionResult>;
  loadAtlas(atlasRef: AtlasReference): Promise<void>;
  getPolicy(scope: string): PolicySet;
}
```

### 2. TRACE Collector (`packages/trace`)

The append-only telemetry system that proves what happened.

**Responsibilities:**
- Event emission and sequencing
- Artifact hashing (SHA-256)
- Stream management (JSONL output)
- Span tracking and correlation
- Replay/diff support
- Golden trace comparison

**Key Interfaces:**
```typescript
interface TRACECollector {
  emit(event: TRACEEvent): void;
  startSpan(spanId: string, metadata: SpanMetadata): void;
  endSpan(spanId: string, result: SpanResult): void;
  getStream(): AsyncIterable<TRACEEvent>;
  replay(traceFile: string): AsyncIterable<TRACEEvent>;
  diff(actual: string, expected: string): TraceDiff;
}
```

### 3. Atlas Loader (`packages/atlas`)

Loads, validates, and manages Atlas packages.

**Responsibilities:**
- Atlas manifest parsing
- Schema validation
- Dependency resolution
- Adapter extraction (platform-specific)
- License verification
- Version management

**Key Interfaces:**
```typescript
interface AtlasLoader {
  load(source: string | AtlasManifest): Promise<Atlas>;
  validate(atlas: Atlas): ValidationResult;
  listAdapters(atlas: Atlas, platform: Platform): Adapter[];
  resolveContext(atlas: Atlas, query: ContextQuery): ContextBlock[];
}
```

### 4. CLI (`packages/cli`)

The primary user interface with asynchronous, telemetry-first UX.

**Responsibilities:**
- Project initialization and scaffolding
- CARP resolve invocation
- TRACE stream tailing
- Configuration management
- Atlas installation

**Commands:**
```
cra init                    # Initialize project (creates agents.md, config/, etc.)
cra resolve <goal>          # Send CARP resolution request
cra execute <action>        # Execute a permitted action
cra trace tail              # Tail TRACE event stream
cra trace replay <file>     # Replay a trace file
cra trace diff <a> <b>      # Diff two trace files
cra atlas install <ref>     # Install an Atlas
cra atlas validate <path>   # Validate an Atlas
cra config set <key> <val>  # Set configuration
```

### 5. Platform Adapters (`packages/adapters`)

Translate CRA artifacts to platform-specific formats.

**Supported Platforms:**
- OpenAI (tool calling, function definitions)
- Claude (tool use, system prompts)
- Google ADK (orchestration primitives)
- MCP (servers, resources, tools)

**Key Interfaces:**
```typescript
interface PlatformAdapter {
  platform: Platform;
  toToolDefinitions(actions: ActionSchema[]): PlatformTool[];
  toSystemPrompt(contextBlocks: ContextBlock[]): string;
  fromToolCall(call: PlatformToolCall): CARPAction;
  wrapExecution(action: CARPAction, executor: Executor): Promise<ExecutionResult>;
}
```

### 6. HTTP Server (`packages/server`)

REST API and WebSocket server for CRA operations.

**Responsibilities:**
- CARP resolve/execute endpoints
- Batch operations (up to 100 per request)
- SSE streaming for long-running resolutions
- WebSocket streaming for TRACE events
- Health checks and discovery

**Key Interfaces:**
```typescript
interface CRAServer {
  start(): Promise<void>;
  stop(): Promise<void>;
  getConfig(): ServerConfig;
}

// REST Endpoints
POST /v1/resolve     // CARP resolution
POST /v1/execute     // Action execution
POST /v1/batch       // Batch operations
POST /v1/stream/resolve  // SSE streaming
GET  /v1/discover    // Agent discovery
GET  /health         // Health check
WS   /v1/trace       // WebSocket trace stream
```

### 7. Storage Layer (`packages/storage`)

Pluggable persistence for resolutions, sessions, and traces.

**Responsibilities:**
- Resolution storage with TTL
- Session state management
- TRACE event persistence
- Transaction support (PostgreSQL)
- Bulk operations

**Key Interfaces:**
```typescript
interface Store {
  // Resolutions
  saveResolution(record: ResolutionRecord): Promise<void>;
  getResolution(id: string): Promise<ResolutionRecord | null>;
  listResolutions(filter?: ResolutionFilter): Promise<ResolutionRecord[]>;

  // Sessions
  saveSession(record: SessionRecord): Promise<void>;
  getSession(id: string): Promise<SessionRecord | null>;

  // Traces
  saveTraceEvents(events: TRACEEvent[]): Promise<void>;
  getTraceEvents(traceId: string): Promise<TRACEEvent[]>;
}

// Factory
function createStore(config: StoreConfig): Store;
type StoreType = 'memory' | 'file' | 'postgresql';
```

### 8. Redaction Engine (`packages/trace`)

Pattern-based and field-level sensitive data redaction.

**Responsibilities:**
- Pattern matching (regex-based)
- Field-level redaction rules
- Multiple redaction modes (full, partial, hash, mask, remove)
- TRACE event redaction
- Sensitive field detection

**Key Interfaces:**
```typescript
interface RedactionEngine {
  redactEvent(event: TRACEEvent): TRACEEvent;
  redactString(value: string): string;
  redactObject<T>(obj: T, basePath?: string): T;
}

// Built-in patterns
type PatternName = 'email' | 'phone' | 'ssn' | 'credit_card' |
                   'api_key' | 'jwt' | 'ip_address' | 'password';

// Redaction modes
type RedactionMode = 'full' | 'partial' | 'hash' | 'mask' | 'remove';
```

### 9. Golden Trace Testing (`packages/trace`)

Record, replay, and validate TRACE sequences for testing.

**Responsibilities:**
- Trace recording from collectors
- Golden trace storage and comparison
- Configurable field ignoring
- Fingerprinting for trace identification
- Test framework integration

**Key Interfaces:**
```typescript
interface GoldenTraceManager {
  startRecording(collector: TRACECollector, name: string): void;
  stopRecording(): GoldenTraceTest;
  registerGolden(name: string, trace: GoldenTraceTest): void;
  compare(name: string, events: TRACEEvent[]): GoldenTraceResult;
  fingerprint(events: TRACEEvent[]): string;
}

interface GoldenTraceAssertion {
  matchesGolden(name: string): GoldenTraceResult;
  hasEventType(type: string): boolean;
  hasEventCount(count: number): boolean;
  recordAsGolden(name: string): GoldenTraceTest;
}
```

### 10. Dual-Mode UI (`packages/ui`)

Human-friendly terminal and agent-optimized JSON interface.

**Responsibilities:**
- Interactive terminal with command history
- Real-time trace visualization
- Agent discovery endpoint
- Structured JSON API for agents
- agents.md snippet generation

**Key Interfaces:**
```typescript
interface UIServer {
  start(): Promise<void>;
  stop(): Promise<void>;
}

interface AgentAPI {
  getDashboardData(): AgentDashboardData;
  getTerminalData(): AgentTerminalData;
  getTracesData(): AgentTracesData;
  getTraceData(traceId: string): AgentTraceData;
  getFullContext(): AgentFullContext;
}

// Agent-optimized response includes
interface AgentFullContext {
  agents_md_snippet: string;  // Ready-to-use agents.md config
  quick_start: string[];
  capabilities: CapabilityInfo[];
}
```

### 11. OpenTelemetry Export (`packages/otel`)

Bridge TRACE events to OpenTelemetry-compatible systems.

**Responsibilities:**
- TRACE to OTel span conversion
- Attribute mapping
- Batch export
- Multiple protocol support (gRPC, HTTP)

**Key Interfaces:**
```typescript
interface OTelExporter {
  export(event: TRACEEvent): void;
  exportBatch(events: TRACEEvent[]): void;
  flush(): Promise<void>;
  shutdown(): Promise<void>;
}

interface OTelConfig {
  endpoint: string;
  serviceName: string;
  protocol: 'grpc' | 'http';
  headers?: Record<string, string>;
}
```

### 12. MCP Integration (`packages/mcp`)

Model Context Protocol server implementation.

**Responsibilities:**
- MCP server lifecycle
- Resource exposure
- Tool registration
- CRA-to-MCP translation

**Key Interfaces:**
```typescript
interface MCPServer {
  start(): Promise<void>;
  stop(): Promise<void>;
  registerResource(resource: MCPResource): void;
  registerTool(tool: MCPTool): void;
}
```

### 13. Registry Service (`packages/registry`) [Future]

Marketplace for Atlas discovery, licensing, and certification.

**Responsibilities:**
- Atlas publishing and versioning
- License management
- Usage metering
- Certification gates
- Revenue handling

---

## Data Models

### CARP Envelope

```typescript
// CARP Request - sent by acting agent
interface CARPRequest {
  carp_version: "1.0";
  request_id: string;              // UUIDv7
  timestamp: string;               // ISO 8601
  operation: "resolve" | "execute" | "validate";

  requester: {
    agent_id: string;
    session_id: string;
    auth_token?: string;
  };

  task: {
    goal: string;                  // Natural language goal
    risk_tier: "low" | "medium" | "high" | "critical";
    context_hints?: string[];      // Requested context domains
    constraints?: Record<string, unknown>;
  };

  scope?: {
    atlases?: string[];            // Limit to specific atlases
    actions?: string[];            // Limit to specific action types
    max_context_tokens?: number;
  };
}

// CARP Resolution - returned by CRA
interface CARPResolution {
  carp_version: "1.0";
  request_id: string;              // Echoes request
  resolution_id: string;           // UUIDv7
  timestamp: string;

  decision: CARPDecision;

  context_blocks: ContextBlock[];
  allowed_actions: ActionPermission[];
  denied_actions: ActionDenial[];

  policies_applied: PolicyReference[];
  evidence: Evidence[];

  ttl: {
    context_expires_at: string;    // ISO 8601
    resolution_expires_at: string;
  };

  telemetry_link: {
    trace_id: string;
    span_id: string;
  };
}

type CARPDecision =
  | { type: "allow" }
  | { type: "allow_with_constraints"; constraints: Constraint[] }
  | { type: "deny"; reason: string; remediation?: string }
  | { type: "requires_approval"; approvers: string[]; timeout: number }
  | { type: "insufficient_context"; missing: string[] };

interface ContextBlock {
  block_id: string;
  atlas_ref: string;
  domain: string;
  content: string;                 // Markdown or structured
  content_hash: string;            // SHA-256
  ttl_seconds: number;
  evidence_refs: string[];
  redactions?: Redaction[];
}

interface ActionPermission {
  action_id: string;
  action_type: string;             // e.g., "api.call", "file.write"
  schema: JSONSchema;
  constraints: Constraint[];
  requires_approval: boolean;
  rate_limit?: RateLimit;
}

interface ActionDenial {
  action_type: string;
  reason: string;
  policy_ref: string;
}

interface Constraint {
  type: string;
  params: Record<string, unknown>;
}

interface Evidence {
  evidence_id: string;
  type: "documentation" | "example" | "test_result" | "policy" | "external";
  source: string;
  content_hash: string;
  url?: string;
}

interface PolicyReference {
  policy_id: string;
  atlas_ref: string;
  name: string;
  version: string;
}

interface Redaction {
  field: string;
  reason: string;
  policy_ref: string;
}
```

### TRACE Event Schema

```typescript
interface TRACEEvent {
  trace_version: "1.0";
  event_id: string;                // UUIDv7
  timestamp: string;               // ISO 8601 with microseconds
  sequence: number;                // Monotonic within session

  trace_id: string;                // Correlation ID for entire operation
  span_id: string;                 // Current span
  parent_span_id?: string;         // Parent span (for nesting)

  event_type: TRACEEventType;

  payload: Record<string, unknown>;

  artifacts?: ArtifactReference[];

  // Integrity
  previous_event_hash?: string;    // SHA-256 of previous event (chain)
  event_hash: string;              // SHA-256 of this event
}

type TRACEEventType =
  // CARP events
  | "carp.request.received"
  | "carp.resolution.started"
  | "carp.context.loaded"
  | "carp.policy.evaluated"
  | "carp.resolution.completed"
  | "carp.action.requested"
  | "carp.action.approved"
  | "carp.action.denied"
  | "carp.action.executed"
  | "carp.action.failed"

  // Atlas events
  | "atlas.loaded"
  | "atlas.validation.started"
  | "atlas.validation.completed"

  // Adapter events
  | "adapter.tool.generated"
  | "adapter.call.received"
  | "adapter.call.translated"

  // System events
  | "session.started"
  | "session.ended"
  | "error.occurred"
  | "warning.raised";

interface ArtifactReference {
  artifact_id: string;
  type: "request" | "response" | "context" | "action" | "result" | "error";
  content_hash: string;
  size_bytes: number;
  storage_ref?: string;            // For large artifacts
}

interface SpanMetadata {
  span_id: string;
  name: string;
  parent_span_id?: string;
  attributes: Record<string, unknown>;
}

interface SpanResult {
  status: "ok" | "error" | "timeout";
  duration_ms: number;
  attributes?: Record<string, unknown>;
}
```

### Atlas Manifest Schema

```typescript
interface AtlasManifest {
  atlas_version: "0.1";

  metadata: {
    id: string;                    // Unique identifier (reverse-dns style)
    name: string;
    version: string;               // SemVer
    description: string;
    authors: Author[];
    license: LicenseInfo;
    keywords: string[];
    homepage?: string;
    repository?: string;
  };

  domains: Domain[];

  context_packs: ContextPack[];

  policies: Policy[];

  actions: ActionDefinition[];

  adapters: AdapterConfig[];

  tests: TestSuite[];

  dependencies?: AtlasDependency[];
}

interface Domain {
  id: string;
  name: string;
  description: string;
  risk_tier: "low" | "medium" | "high" | "critical";
}

interface ContextPack {
  id: string;
  domain: string;
  source: string;                  // Path to markdown/json file
  ttl_seconds: number;
  tags: string[];
  evidence_sources: string[];
}

interface Policy {
  id: string;
  name: string;
  version: string;
  rules: PolicyRule[];
}

interface PolicyRule {
  id: string;
  description: string;
  condition: PolicyCondition;
  effect: "allow" | "deny" | "require_approval" | "redact";
  priority: number;
}

interface PolicyCondition {
  type: "risk_tier" | "action_type" | "domain" | "requester" | "time" | "rate" | "custom";
  operator: "eq" | "neq" | "in" | "not_in" | "gt" | "lt" | "matches";
  value: unknown;
}

interface ActionDefinition {
  id: string;
  type: string;
  name: string;
  description: string;
  domain: string;
  schema: JSONSchema;
  examples: ActionExample[];
  risk_tier: "low" | "medium" | "high" | "critical";
}

interface AdapterConfig {
  platform: "openai" | "claude" | "google_adk" | "mcp";
  config_file: string;             // Path to platform-specific config
  tool_mappings?: ToolMapping[];
}

interface ToolMapping {
  action_id: string;
  platform_tool_name: string;
  parameter_transforms?: Record<string, string>;
}

interface TestSuite {
  id: string;
  name: string;
  type: "unit" | "integration" | "conformance" | "golden_trace";
  test_files: string[];
}

interface Author {
  name: string;
  email?: string;
  url?: string;
}

interface LicenseInfo {
  type: "free" | "paid" | "subscription" | "usage" | "enterprise";
  spdx?: string;                   // For open source
  terms_url?: string;
  price?: PriceInfo;
}

interface PriceInfo {
  model: "one_time" | "monthly" | "yearly" | "per_resolution" | "per_action";
  amount_cents: number;
  currency: string;
}

interface AtlasDependency {
  atlas_id: string;
  version_range: string;           // SemVer range
}
```

---

## Extension Points

### 1. Custom Policy Conditions

Extend policy evaluation with custom condition types:

```typescript
interface PolicyConditionHandler {
  type: string;
  evaluate(condition: PolicyCondition, context: EvaluationContext): boolean;
}

runtime.registerConditionHandler({
  type: "custom_compliance_check",
  evaluate: (condition, context) => {
    return myComplianceSystem.check(context.requester, condition.value);
  }
});
```

### 2. Custom Evidence Sources

Add new evidence types:

```typescript
interface EvidenceProvider {
  type: string;
  fetch(ref: string): Promise<Evidence>;
}

runtime.registerEvidenceProvider({
  type: "confluence",
  fetch: async (ref) => {
    const doc = await confluence.getDocument(ref);
    return { type: "documentation", source: ref, content_hash: hash(doc) };
  }
});
```

### 3. Custom Platform Adapters

Add support for new platforms:

```typescript
interface PlatformAdapterFactory {
  platform: string;
  create(config: AdapterConfig): PlatformAdapter;
}

adapters.register({
  platform: "custom_orchestrator",
  create: (config) => new CustomOrchestratorAdapter(config)
});
```

### 4. TRACE Event Processors

Add custom event processing:

```typescript
interface TRACEEventProcessor {
  name: string;
  process(event: TRACEEvent): void | Promise<void>;
}

trace.addProcessor({
  name: "audit_logger",
  process: async (event) => {
    await auditSystem.log(event);
  }
});
```

---

## Scaling Path

### Phase 1: Single-Node (MVP)

```
┌─────────────────────────────────────────┐
│              Developer Machine          │
├─────────────────────────────────────────┤
│  CLI ──► Runtime ──► TRACE (file)      │
│              │                          │
│              ▼                          │
│         Local Atlas Store               │
└─────────────────────────────────────────┘
```

- Single process runtime
- File-based TRACE storage
- Local Atlas loading
- CLI-only interface

### Phase 2: Multi-Tenant SaaS

```
┌─────────────────────────────────────────────────────────────────┐
│                        Cloud Platform                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────┐    ┌────────────────┐    ┌───────────────┐ │
│  │   API Gateway  │───►│ Runtime Cluster │───►│ TRACE Store   │ │
│  │   (Auth/Rate)  │    │  (Kubernetes)   │    │ (TimescaleDB) │ │
│  └────────────────┘    └────────────────┘    └───────────────┘ │
│          │                     │                                 │
│          │                     ▼                                 │
│          │             ┌────────────────┐    ┌───────────────┐ │
│          │             │  Atlas Registry │───►│   Blob Store  │ │
│          │             │   (Postgres)    │    │    (S3)       │ │
│          │             └────────────────┘    └───────────────┘ │
│          │                                                       │
│          ▼                                                       │
│  ┌────────────────────────────────────────────────────────────┐│
│  │                    Tenant Isolation Layer                   ││
│  └────────────────────────────────────────────────────────────┘│
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

- Horizontal scaling via Kubernetes
- Tenant isolation
- Centralized Atlas registry
- Usage metering
- Web dashboard

### Phase 3: Airgapped Enterprise

```
┌─────────────────────────────────────────────────────────────────┐
│                     Enterprise Datacenter                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                  CRA Enterprise Cluster                   │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │  │
│  │  │ Runtime Pod │  │ Runtime Pod │  │ Audit/Compliance │  │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                   │
│              ┌───────────────┼───────────────┐                  │
│              ▼               ▼               ▼                  │
│  ┌─────────────────┐ ┌─────────────┐ ┌─────────────────────┐  │
│  │ Local Atlas Repo │ │ TRACE SIEM  │ │ LDAP/SSO Integration│  │
│  │ (Artifactory)    │ │ Integration │ │                     │  │
│  └─────────────────┘ └─────────────┘ └─────────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

- On-premises deployment
- LDAP/SSO integration
- SIEM integration for TRACE
- Local Atlas repository
- Full audit trail
- Compliance reporting

### Phase 4: Edge/Robotics

```
┌─────────────────────────────────────────────────────────────────┐
│                        Edge Device                               │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              CRA Edge Runtime (Embedded)                 │   │
│  │  ┌───────────────┐  ┌────────────┐  ┌───────────────┐  │   │
│  │  │ Minimal CARP  │  │ Local TRACE│  │ Cached Atlas  │  │   │
│  │  │ Resolver      │  │ Buffer     │  │ (Read-only)   │  │   │
│  │  └───────────────┘  └────────────┘  └───────────────┘  │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                    Sync when connected                          │
│                              │                                   │
│                              ▼                                   │
│                     ┌─────────────────┐                         │
│                     │  Cloud Backend  │                         │
│                     └─────────────────┘                         │
└─────────────────────────────────────────────────────────────────┘
```

- Minimal runtime footprint
- Offline-first operation
- Sync when connected
- Pre-loaded Atlas bundles
- Bounded TRACE buffer

---

## Security Model

### Authentication

```typescript
interface AuthContext {
  // Identity
  agent_id: string;
  session_id: string;

  // Credentials
  auth_method: "api_key" | "jwt" | "mtls" | "oauth2";
  token_claims?: Record<string, unknown>;

  // Trust level
  trust_level: "anonymous" | "authenticated" | "verified" | "privileged";

  // Organization (for multi-tenant)
  org_id?: string;
  team_id?: string;
}
```

### Authorization Scopes

```typescript
type Scope =
  | "carp:resolve"           // Can request resolutions
  | "carp:execute"           // Can execute actions
  | "carp:execute:high_risk" // Can execute high-risk actions
  | "trace:read"             // Can read trace events
  | "trace:replay"           // Can replay traces
  | "atlas:read"             // Can read atlas contents
  | "atlas:install"          // Can install atlases
  | "atlas:publish"          // Can publish atlases
  | "admin:policy"           // Can modify policies
  | "admin:users";           // Can manage users
```

### Rate Limiting

```typescript
interface RateLimit {
  window_seconds: number;
  max_requests: number;
  scope: "global" | "per_agent" | "per_action" | "per_atlas";
  burst_allowance?: number;
}
```

### Redaction

Sensitive data is automatically redacted based on policies:

```typescript
interface RedactionPolicy {
  id: string;
  patterns: RegexPattern[];
  fields: string[];
  replacement: string;  // e.g., "[REDACTED]"
  audit_log: boolean;   // Log that redaction occurred
}
```

---

## Performance Considerations

### Caching Strategy

1. **Atlas Cache**: LRU cache for loaded atlases (10 min default TTL)
2. **Policy Cache**: Pre-computed policy decision trees
3. **Context Cache**: Hash-based deduplication of context blocks
4. **Resolution Cache**: Short-lived cache for identical requests (30s)

### Async Processing

- TRACE events are buffered and written asynchronously
- Large artifact storage is non-blocking
- Policy evaluation uses parallel rule evaluation where possible

### Resource Limits

```typescript
interface RuntimeLimits {
  max_context_tokens: number;      // Default: 8192
  max_actions_per_resolution: number; // Default: 50
  max_evidence_items: number;      // Default: 20
  resolution_timeout_ms: number;   // Default: 5000
  trace_buffer_size: number;       // Default: 10000 events
}
```

---

## Observability

### Metrics (Prometheus)

- `cra_resolutions_total{decision, atlas, risk_tier}`
- `cra_resolution_latency_seconds{atlas}`
- `cra_actions_executed_total{action_type, result}`
- `cra_trace_events_total{event_type}`
- `cra_atlas_loads_total{atlas_id}`
- `cra_policy_evaluations_total{policy_id, effect}`

### Logs

Structured JSON logs with correlation IDs:

```json
{
  "level": "info",
  "message": "Resolution completed",
  "trace_id": "...",
  "span_id": "...",
  "resolution_id": "...",
  "decision": "allow_with_constraints",
  "duration_ms": 42
}
```

### Health Checks

- `/health/live` - Liveness (process running)
- `/health/ready` - Readiness (dependencies connected)
- `/health/startup` - Startup (initial load complete)

---

## Error Handling

### Error Types

```typescript
type CRAError =
  | { code: "CARP_INVALID_REQUEST"; message: string; details: ValidationError[] }
  | { code: "CARP_ATLAS_NOT_FOUND"; atlas_id: string }
  | { code: "CARP_POLICY_VIOLATION"; policy_id: string; reason: string }
  | { code: "CARP_RESOLUTION_TIMEOUT"; timeout_ms: number }
  | { code: "CARP_ACTION_DENIED"; action_id: string; reason: string }
  | { code: "TRACE_WRITE_FAILED"; reason: string }
  | { code: "ATLAS_VALIDATION_FAILED"; errors: ValidationError[] }
  | { code: "AUTH_FAILED"; reason: string }
  | { code: "RATE_LIMITED"; retry_after_seconds: number };
```

### Recovery Strategies

1. **Transient failures**: Exponential backoff with jitter
2. **Atlas load failures**: Fallback to cached version
3. **TRACE write failures**: Buffer locally, retry
4. **Policy evaluation errors**: Fail closed (deny)

---

## Testing Strategy

See `docs/TESTING.md` for detailed conformance tests and golden trace approach.
