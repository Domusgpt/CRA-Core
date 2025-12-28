# CRA Improvement Plan

## Current State Assessment

### What We Have (v0.1)
- ✅ Core packages: protocol, trace, atlas, runtime, adapters, cli
- ✅ CARP/TRACE protocol specifications
- ✅ 76 passing tests (protocol: 28, trace: 26, runtime: 22)
- ✅ Build system working
- ✅ Reference atlas (github-ops)
- ✅ Basic CLI commands

### Gaps Identified
- ❌ No HTTP server mode (CLI only)
- ❌ No authentication/authorization
- ❌ No rate limiting
- ❌ Platform adapters are stubs (not functional)
- ❌ No real action execution (simulated)
- ❌ Missing tests for adapters, atlas, cli packages
- ❌ No MCP server implementation
- ❌ No streaming API
- ❌ No persistence layer
- ❌ No metrics/monitoring

---

## Phase 1: Core Infrastructure (Priority: Critical)

### 1.1 HTTP Server Mode
**Goal**: Enable CRA to run as a service accepting HTTP requests

```typescript
// packages/server/src/server.ts
- POST /v1/resolve - CARP resolution endpoint
- POST /v1/execute - Action execution endpoint
- GET /v1/health - Health check
- GET /v1/stats - Runtime statistics
- WS /v1/trace - WebSocket trace streaming
```

**Files to create**:
- `packages/server/` - New server package
- Express/Fastify HTTP server
- WebSocket support for trace streaming
- OpenAPI specification

### 1.2 Persistence Layer
**Goal**: Store resolutions, traces, and session state

```typescript
// packages/storage/src/
- ResolutionStore - Cache resolutions with TTL
- TraceStore - Persist traces to disk/database
- SessionStore - Track active sessions
```

**Options**:
- SQLite for local development
- PostgreSQL for production
- S3/GCS for trace archives

### 1.3 Real Platform Adapters
**Goal**: Functional adapters for major platforms

**OpenAI Adapter**:
- Convert CARP actions to OpenAI function definitions
- Parse function_call responses back to CARP actions
- Handle tool_choice constraints

**Anthropic/Claude Adapter**:
- Convert to Claude tool format
- Handle tool_use blocks
- Support streaming responses

**MCP Adapter**:
- Implement MCP server protocol
- Expose CRA as MCP tools
- Handle prompts and resources

---

## Phase 2: Security & Governance (Priority: High)

### 2.1 Authentication
**Goal**: Secure API access

```typescript
// packages/auth/src/
- API key authentication
- JWT token support
- OAuth2 integration (optional)
- Per-agent identity verification
```

### 2.2 Rate Limiting
**Goal**: Prevent abuse and enforce quotas

```typescript
// packages/governance/src/rate-limiter.ts
- Per-agent rate limits
- Per-atlas rate limits
- Token bucket algorithm
- Sliding window counters
```

### 2.3 Redaction Engine
**Goal**: Protect sensitive data in traces

```typescript
// packages/governance/src/redaction.ts
- Pattern-based redaction (SSN, credit cards, etc.)
- Policy-driven redaction rules
- Audit log for redacted content
```

### 2.4 Audit Logging
**Goal**: Comprehensive audit trail

```typescript
// All actions logged with:
- Who (agent_id, session_id)
- What (action, parameters)
- When (timestamp)
- Why (resolution_id, policy_refs)
- Outcome (success/failure)
```

---

## Phase 3: Developer Experience (Priority: High)

### 3.1 Enhanced CLI
**Goal**: Better developer experience

```bash
# New commands
cra serve                    # Start HTTP server
cra serve --port 8080        # Custom port
cra atlas create <name>      # Scaffold new atlas
cra atlas publish            # Publish to registry
cra trace export <file>      # Export as JSON/CSV
cra trace visualize <file>   # Open trace viewer
cra doctor                   # Diagnose issues
```

### 3.2 Atlas Development Kit
**Goal**: Easy atlas creation

```typescript
// packages/adk/src/
- Atlas scaffolding templates
- Context pack generators
- Policy validators
- Action schema generators
- Hot reload for development
```

### 3.3 Trace Viewer UI
**Goal**: Visual trace analysis

```typescript
// packages/trace-viewer/
- Web-based trace visualization
- Timeline view
- Span hierarchy
- Event filtering
- Diff comparison
```

---

## Phase 4: Advanced Features (Priority: Medium)

### 4.1 Streaming Resolution
**Goal**: Stream context and actions progressively

```typescript
// Stream chunks as they become available
for await (const chunk of runtime.resolveStream(request)) {
  if (chunk.type === 'context') yield chunk.block;
  if (chunk.type === 'action') yield chunk.permission;
}
```

### 4.2 Multi-Agent Coordination
**Goal**: Support agent-to-agent communication

```typescript
// packages/coordination/src/
- Agent registry
- Handoff protocol
- Shared context
- Conflict resolution
```

### 4.3 Atlas Marketplace
**Goal**: Discover and share atlases

```typescript
// packages/registry/src/
- Atlas registry API
- Version management
- Dependency resolution
- License enforcement
```

### 4.4 Golden Trace Testing
**Goal**: Regression testing with expected traces

```typescript
// Compare actual trace to golden trace
const diff = diffTraces(actualEvents, goldenEvents);
expect(diff.compatibility).toBe('identical');
```

---

## Phase 5: Production Readiness (Priority: Medium)

### 5.1 Observability
**Goal**: Production monitoring

```typescript
// Metrics (Prometheus compatible)
- cra_resolutions_total
- cra_resolution_duration_seconds
- cra_actions_total
- cra_cache_hit_ratio
- cra_trace_events_total

// Distributed tracing
- OpenTelemetry integration
- Trace context propagation
```

### 5.2 High Availability
**Goal**: Reliable production deployment

```typescript
// packages/cluster/src/
- Multi-node coordination
- Leader election
- State replication
- Graceful shutdown
```

### 5.3 Performance Optimization
**Goal**: Low latency resolution

- Resolution caching
- Context pre-computation
- Connection pooling
- Batch processing

---

## Implementation Priority Matrix

| Feature | Impact | Effort | Priority |
|---------|--------|--------|----------|
| HTTP Server | High | Medium | P0 |
| Real OpenAI Adapter | High | Low | P0 |
| Real Claude Adapter | High | Low | P0 |
| Missing Package Tests | Medium | Low | P0 |
| Authentication | High | Medium | P1 |
| Rate Limiting | Medium | Low | P1 |
| Enhanced CLI | Medium | Medium | P1 |
| MCP Server | High | Medium | P1 |
| Persistence Layer | Medium | Medium | P2 |
| Trace Viewer | Medium | High | P2 |
| Streaming Resolution | Medium | Medium | P2 |
| Atlas Marketplace | Low | High | P3 |
| Multi-Agent Coord | Low | High | P3 |

---

## Immediate Next Steps (This Session)

1. **Add missing tests** for adapters, atlas, cli packages
2. **Implement real OpenAI adapter** with function calling
3. **Implement real Claude adapter** with tool_use
4. **Create HTTP server package** with basic endpoints
5. **Add authentication middleware** with API keys

---

## Success Metrics

### v0.2 Goals
- [ ] HTTP server with /resolve and /execute endpoints
- [ ] Functional OpenAI and Claude adapters
- [ ] 100+ tests with >80% code coverage
- [ ] API key authentication
- [ ] Rate limiting (100 req/min default)

### v0.3 Goals
- [ ] MCP server implementation
- [ ] Trace viewer UI
- [ ] Atlas scaffolding CLI
- [ ] PostgreSQL persistence
- [ ] OpenTelemetry integration

### v1.0 Goals
- [ ] Production-ready deployment
- [ ] Atlas marketplace beta
- [ ] Multi-agent coordination
- [ ] SOC2 compliance ready
