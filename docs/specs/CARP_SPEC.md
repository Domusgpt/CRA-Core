# CARP/1.0 Specification

## Context & Action Resolution Protocol

**Version**: 1.0
**Status**: Draft
**Last Updated**: 2025-01-01

---

## 1. Introduction

CARP (Context & Action Resolution Protocol) defines a deterministic contract between an **acting agent** (Requester) and a **context authority** (CRA Resolver). It establishes what context may be injected, what actions are permitted, and under what constraints—all with evidence and telemetry linkage.

### 1.1 Design Principles

1. **Runtime Authority**: The CRA runtime is authoritative; LLM output is advisory
2. **Least Privilege Context**: Only provide context necessary for the task
3. **Explicit Permissions**: Actions must be explicitly allowed, not implicitly assumed
4. **Evidence-Backed**: All context and permissions link to verifiable evidence
5. **TTL-Bounded**: All context has expiration; stale context must be re-resolved
6. **Telemetry-Linked**: Every resolution produces TRACE events

### 1.2 Transport Independence

CARP is transport-agnostic. It can be carried over:
- HTTP/REST
- WebSocket
- gRPC
- MCP (Model Context Protocol)
- In-process function calls

---

## 2. Protocol Operations

### 2.1 Resolve

Request context and action permissions for a given task.

```
Requester                            Resolver
    |                                    |
    |  ────── CARPRequest ──────────►   |
    |                                    |
    |  ◄───── CARPResolution ────────   |
    |                                    |
```

### 2.2 Execute

Request execution of a previously-permitted action.

```
Requester                            Resolver
    |                                    |
    |  ────── CARPActionRequest ────►   |
    |                                    |
    |  ◄───── CARPExecutionResult ───   |
    |                                    |
```

### 2.3 Validate

Validate a proposed action without executing it.

```
Requester                            Resolver
    |                                    |
    |  ────── CARPValidateRequest ──►   |
    |                                    |
    |  ◄───── CARPValidationResult ──   |
    |                                    |
```

---

## 3. Message Formats

### 3.1 CARPRequest

```typescript
interface CARPRequest {
  // Protocol version
  carp_version: "1.0";

  // Request identification
  request_id: string;              // UUIDv7, client-generated
  timestamp: string;               // ISO 8601 with timezone

  // Operation type
  operation: "resolve" | "execute" | "validate";

  // Requester identity
  requester: {
    agent_id: string;              // Unique agent identifier
    agent_type?: string;           // e.g., "claude", "gpt-4", "custom"
    session_id: string;            // Session correlation
    auth_token?: string;           // Bearer token or API key
    metadata?: Record<string, string>;
  };

  // Task specification (for resolve)
  task?: {
    goal: string;                  // Natural language description
    goal_hash?: string;            // SHA-256 of goal for caching
    risk_tier?: RiskTier;          // Declared risk level
    context_hints?: string[];      // Requested domains/topics
    constraints?: TaskConstraint[];
    parent_task_id?: string;       // For subtask chaining
  };

  // Action specification (for execute/validate)
  action?: {
    action_id: string;             // From prior resolution
    action_type: string;           // Action type identifier
    parameters: Record<string, unknown>;
    resolution_id: string;         // Reference to resolution
  };

  // Scoping
  scope?: {
    atlases?: string[];            // Limit to specific atlases
    domains?: string[];            // Limit to specific domains
    actions?: string[];            // Limit action types
    max_context_tokens?: number;   // Context size limit
    max_actions?: number;          // Action count limit
  };

  // Telemetry
  telemetry?: {
    trace_id?: string;             // Existing trace to join
    parent_span_id?: string;       // Parent span
    sampling_rate?: number;        // 0.0 - 1.0
  };
}

type RiskTier = "low" | "medium" | "high" | "critical";

interface TaskConstraint {
  type: string;
  value: unknown;
}
```

### 3.2 CARPResolution

```typescript
interface CARPResolution {
  // Protocol version
  carp_version: "1.0";

  // Response identification
  request_id: string;              // Echoes request
  resolution_id: string;           // UUIDv7, server-generated
  timestamp: string;               // ISO 8601

  // Decision
  decision: CARPDecision;

  // Context blocks (if allowed)
  context_blocks: ContextBlock[];

  // Permitted actions
  allowed_actions: ActionPermission[];

  // Explicitly denied actions (with reasons)
  denied_actions: ActionDenial[];

  // Policy trace
  policies_applied: PolicyApplication[];

  // Supporting evidence
  evidence: Evidence[];

  // Time bounds
  ttl: {
    context_expires_at: string;    // ISO 8601
    resolution_expires_at: string; // ISO 8601
    refresh_hint_seconds?: number; // Suggested refresh interval
  };

  // Telemetry linkage
  telemetry_link: {
    trace_id: string;
    span_id: string;
    events_emitted: number;
  };

  // Warnings (non-fatal issues)
  warnings?: Warning[];
}

// Decision types
type CARPDecision =
  | AllowDecision
  | AllowWithConstraintsDecision
  | DenyDecision
  | RequiresApprovalDecision
  | InsufficientContextDecision
  | PartialDecision;

interface AllowDecision {
  type: "allow";
}

interface AllowWithConstraintsDecision {
  type: "allow_with_constraints";
  constraints: Constraint[];
}

interface DenyDecision {
  type: "deny";
  reason: string;
  policy_refs: string[];
  remediation?: string;
}

interface RequiresApprovalDecision {
  type: "requires_approval";
  approvers: Approver[];
  approval_timeout_seconds: number;
  approval_url?: string;
}

interface InsufficientContextDecision {
  type: "insufficient_context";
  missing_domains: string[];
  missing_atlases?: string[];
  suggestion?: string;
}

interface PartialDecision {
  type: "partial";
  reason: string;
  allowed_subset: string[];
  denied_subset: string[];
}

interface Approver {
  id: string;
  type: "user" | "role" | "system";
  name?: string;
}

interface Constraint {
  id: string;
  type: ConstraintType;
  description: string;
  params: Record<string, unknown>;
  enforcement: "hard" | "soft";
}

type ConstraintType =
  | "rate_limit"
  | "time_window"
  | "parameter_restriction"
  | "output_filter"
  | "approval_required"
  | "audit_required"
  | "sandbox"
  | "custom";
```

### 3.3 ContextBlock

```typescript
interface ContextBlock {
  // Identification
  block_id: string;                // Unique within resolution
  content_hash: string;            // SHA-256 of content

  // Source
  atlas_ref: string;               // Atlas ID + version
  pack_ref: string;                // Context pack ID
  domain: string;                  // Domain classification

  // Content
  content_type: "markdown" | "json" | "yaml" | "text";
  content: string;                 // The actual context
  token_count: number;             // Estimated tokens

  // Governance
  ttl_seconds: number;             // How long this context is valid
  classification?: string;         // e.g., "internal", "confidential"
  redactions?: Redaction[];        // Applied redactions

  // Evidence linkage
  evidence_refs: string[];         // IDs of supporting evidence

  // Metadata
  tags: string[];
  priority: number;                // For ordering (higher = more important)
}

interface Redaction {
  original_hash: string;           // Hash before redaction
  redacted_fields: string[];       // What was redacted
  reason: string;                  // Why it was redacted
  policy_ref: string;              // Policy that triggered redaction
}
```

### 3.4 ActionPermission

```typescript
interface ActionPermission {
  // Identification
  action_id: string;               // Unique within resolution
  action_type: string;             // Type identifier (e.g., "api.github.create_issue")

  // Description
  name: string;                    // Human-readable name
  description: string;             // What this action does

  // Schema
  schema: JSONSchema;              // Parameter schema
  examples?: ActionExample[];

  // Constraints
  constraints: Constraint[];
  parameter_constraints?: ParameterConstraint[];

  // Governance
  requires_approval: boolean;
  approval_type?: "sync" | "async";
  risk_tier: RiskTier;

  // Rate limiting
  rate_limit?: RateLimit;

  // Source
  atlas_ref: string;
  evidence_refs: string[];

  // Validity
  valid_until: string;             // ISO 8601
}

interface ParameterConstraint {
  parameter: string;               // JSON path
  constraint_type: "enum" | "range" | "pattern" | "max_length" | "required" | "forbidden";
  value: unknown;
  message: string;
}

interface ActionExample {
  description: string;
  parameters: Record<string, unknown>;
  expected_outcome?: string;
}

interface RateLimit {
  requests: number;
  window_seconds: number;
  scope: "action" | "session" | "agent" | "global";
  current_usage?: number;
  resets_at?: string;
}
```

### 3.5 ActionDenial

```typescript
interface ActionDenial {
  action_type: string;
  reason: string;
  policy_refs: string[];
  permanent: boolean;              // Can this be remediated?
  remediation?: string;            // How to fix
  alternative?: string;            // Suggested alternative action
}
```

### 3.6 Evidence

```typescript
interface Evidence {
  evidence_id: string;
  type: EvidenceType;

  // Source
  source: string;                  // Where this came from
  source_url?: string;             // If web-accessible
  atlas_ref?: string;              // If from atlas

  // Content
  content_hash: string;            // SHA-256
  content_preview?: string;        // First 200 chars
  full_content_ref?: string;       // Storage reference for full content

  // Metadata
  created_at: string;
  verified: boolean;
  verification_method?: string;
}

type EvidenceType =
  | "documentation"                // Official docs
  | "api_spec"                     // OpenAPI/Swagger
  | "example"                      // Usage example
  | "test_result"                  // Test output
  | "policy"                       // Policy document
  | "changelog"                    // Version history
  | "external"                     // Third-party source
  | "user_provided";               // User-supplied
```

### 3.7 PolicyApplication

```typescript
interface PolicyApplication {
  policy_id: string;
  policy_name: string;
  policy_version: string;
  atlas_ref: string;

  rules_evaluated: number;
  rules_matched: number;

  effects: PolicyEffect[];
  evaluation_time_ms: number;
}

interface PolicyEffect {
  rule_id: string;
  effect: "allow" | "deny" | "require_approval" | "redact" | "constrain";
  target: string;                  // What was affected
  reason?: string;
}
```

---

## 4. Execution Messages

### 4.1 CARPActionRequest

```typescript
interface CARPActionRequest {
  carp_version: "1.0";
  request_id: string;
  timestamp: string;
  operation: "execute";

  requester: RequesterInfo;

  action: {
    action_id: string;             // From resolution
    resolution_id: string;         // Must be valid/non-expired
    action_type: string;
    parameters: Record<string, unknown>;
  };

  execution_options?: {
    timeout_ms?: number;
    dry_run?: boolean;
    capture_output?: boolean;
    sandbox?: boolean;
  };

  telemetry?: TelemetryOptions;
}
```

### 4.2 CARPExecutionResult

```typescript
interface CARPExecutionResult {
  carp_version: "1.0";
  request_id: string;
  execution_id: string;
  timestamp: string;

  status: ExecutionStatus;

  result?: {
    output: unknown;               // Action output
    output_hash: string;           // SHA-256 of output
    output_type: string;           // MIME type
  };

  error?: {
    code: string;
    message: string;
    details?: Record<string, unknown>;
    retriable: boolean;
  };

  metrics: {
    duration_ms: number;
    tokens_used?: number;
    api_calls?: number;
  };

  side_effects?: SideEffect[];

  telemetry_link: TelemetryLink;
}

type ExecutionStatus =
  | "success"
  | "partial_success"
  | "failed"
  | "timeout"
  | "cancelled"
  | "pending_approval";

interface SideEffect {
  type: string;                    // e.g., "file_created", "api_called"
  target: string;                  // What was affected
  reversible: boolean;
  details?: Record<string, unknown>;
}
```

---

## 5. Error Handling

### 5.1 Error Response

```typescript
interface CARPError {
  carp_version: "1.0";
  request_id: string;
  timestamp: string;

  error: {
    code: CARPErrorCode;
    message: string;
    details?: Record<string, unknown>;
    trace_id?: string;
  };

  retry?: {
    retriable: boolean;
    retry_after_seconds?: number;
    max_retries?: number;
  };
}

type CARPErrorCode =
  // Request errors
  | "INVALID_REQUEST"
  | "INVALID_VERSION"
  | "MISSING_FIELD"
  | "INVALID_FORMAT"

  // Auth errors
  | "UNAUTHORIZED"
  | "FORBIDDEN"
  | "TOKEN_EXPIRED"

  // Resolution errors
  | "ATLAS_NOT_FOUND"
  | "DOMAIN_NOT_FOUND"
  | "RESOLUTION_EXPIRED"
  | "RESOLUTION_NOT_FOUND"

  // Execution errors
  | "ACTION_NOT_PERMITTED"
  | "ACTION_DENIED"
  | "CONSTRAINT_VIOLATED"
  | "EXECUTION_FAILED"
  | "TIMEOUT"

  // Rate limiting
  | "RATE_LIMITED"

  // System errors
  | "INTERNAL_ERROR"
  | "SERVICE_UNAVAILABLE";
```

---

## 6. Resolution Algorithm

The CRA resolver follows this algorithm:

```
1. VALIDATE request format
   └─ Return INVALID_REQUEST if malformed

2. AUTHENTICATE requester
   └─ Return UNAUTHORIZED if invalid
   └─ Return FORBIDDEN if insufficient permissions

3. LOAD applicable atlases
   ├─ Based on scope.atlases if specified
   ├─ Based on task.context_hints domains
   └─ Return ATLAS_NOT_FOUND if none available

4. EVALUATE policies (in priority order)
   ├─ Check deny rules first
   ├─ Check approval requirements
   ├─ Check constraints
   └─ Accumulate effects

5. ASSEMBLE context blocks
   ├─ Select relevant packs
   ├─ Apply TTL bounds
   ├─ Apply redactions
   └─ Compute hashes

6. DETERMINE permitted actions
   ├─ Match action types to task
   ├─ Apply policy constraints
   └─ Set rate limits

7. GATHER evidence
   ├─ Link documentation
   ├─ Link examples
   └─ Compute content hashes

8. EMIT TRACE events
   └─ Record full resolution trace

9. RETURN resolution
```

---

## 7. Caching

### 7.1 Resolution Caching

Resolutions MAY be cached based on:
- `goal_hash` (SHA-256 of goal text)
- `requester.agent_id`
- `scope` parameters

Cache keys: `{goal_hash}:{agent_id}:{scope_hash}`

Cache TTL: Minimum of all `ttl` values in resolution

### 7.2 Cache Invalidation

Caches MUST be invalidated when:
- Atlas is updated
- Policy is modified
- TTL expires
- Explicit invalidation request

---

## 8. Wire Formats

### 8.1 HTTP

```http
POST /carp/v1/resolve HTTP/1.1
Host: cra.example.com
Content-Type: application/json
Authorization: Bearer <token>
X-Request-ID: <uuid>
X-Trace-ID: <trace_id>

{
  "carp_version": "1.0",
  ...
}
```

Response:
```http
HTTP/1.1 200 OK
Content-Type: application/json
X-Request-ID: <uuid>
X-Resolution-ID: <uuid>
X-Trace-ID: <trace_id>

{
  "carp_version": "1.0",
  ...
}
```

### 8.2 MCP Transport

CARP can be transported over MCP as a tool:

```json
{
  "name": "carp_resolve",
  "description": "Resolve context and permissions via CARP",
  "inputSchema": { ... CARPRequest schema ... }
}
```

---

## 9. Conformance Requirements

### 9.1 MUST Requirements

1. MUST include `carp_version` in all messages
2. MUST generate unique `request_id` / `resolution_id` using UUIDv7
3. MUST validate all requests before processing
4. MUST emit TRACE events for all operations
5. MUST enforce TTL on resolutions
6. MUST hash all content using SHA-256
7. MUST apply policies in priority order

### 9.2 SHOULD Requirements

1. SHOULD cache resolutions when safe
2. SHOULD include evidence for all context blocks
3. SHOULD provide remediation hints for denials
4. SHOULD include rate limit headers in HTTP responses

### 9.3 MAY Requirements

1. MAY support additional transports beyond HTTP
2. MAY implement custom policy conditions
3. MAY extend evidence types
