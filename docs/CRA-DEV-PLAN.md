# CRA Development Plan

## Overview

This document tracks what needs to be designed, implemented, and tested to complete CRA.

---

## Current State

### What Exists (Rust Core)

```
cra-core/src/
├── carp/           # CARP protocol (partially implemented)
│   ├── resolver.rs # Context matching
│   ├── request.rs  # Request structs
│   ├── policy.rs   # Policy evaluation
│   └── resolution.rs
├── trace/          # TRACE protocol (partially implemented)
│   ├── event.rs    # Event + compute_hash()
│   ├── collector.rs # Event collection
│   ├── chain.rs    # Chain verification
│   └── buffer.rs   # Ring buffer
├── atlas/          # Atlas loading (implemented)
│   ├── manifest.rs # Schema definitions
│   ├── loader.rs   # JSON loading
│   └── validator.rs
├── timing/         # Timer backends
└── storage/        # Storage backends
```

### What Exists (Documentation)

| Document | Status |
|----------|--------|
| CRA-COMPLETE-SYSTEM-DESIGN.md | ✅ Done |
| CRA-PITCH.md | ✅ Done |
| CRA-ELEVATOR-PITCH.md | ✅ Done |
| CRA-WRAPPER-SYSTEM.md | ✅ Done |
| CRA-MASTER-PLAN.md | ✅ Done |
| CRA-TRACE-ASYNC.md | ✅ Done |
| MCP-INTEGRATION-DESIGN.md | ✅ Done |
| CRA-BOOTSTRAP-PROTOCOL.md | ✅ Done |
| CRA-STEWARD-CONFIG.md | ✅ Done |
| CRA-AUTHENTICATION.md | ✅ Done |
| CRA-ARCHITECTURE-SIMPLIFIED.md | ✅ Done |
| CRA-WRAPPER-PROTOCOL.md | ✅ Done |
| CRA-CHECKPOINT-SYSTEM.md | ✅ Done |
| CRA-TRACE-EVENTS.md | ✅ Done |
| CRA-ATLAS-SCHEMA-V2.md | ✅ Done |
| CRA-ECOSYSTEM.md | ✅ Done |

### What Exists (Atlases)

| Atlas | Status |
|-------|--------|
| vib3-webpage-development.json | ✅ Done |
| cra-bootstrap.json | ✅ Done |
| cra-agent-integration.json | ✅ Done |

---

## Phase 1: Complete Design Documentation ✅ COMPLETE

| Document | Status | Description |
|----------|--------|-------------|
| `CRA-WRAPPER-PROTOCOL.md` | ✅ | Wrapper structure, platform implementations, plugins |
| `CRA-CHECKPOINT-SYSTEM.md` | ✅ | Checkpoint types, triggers, evaluation, handlers |
| `CRA-TRACE-EVENTS.md` | ✅ | Event types, fields, async/sync, extensibility |
| `CRA-ATLAS-SCHEMA-V2.md` | ✅ | Full schema with steward, marketplace config |
| `CRA-ECOSYSTEM.md` | ✅ | Plugins, marketplace, skills, creator platform |

---

## Phase 2: Rust Core Implementation

### 2.1 Async TRACE

**Status:** Not implemented (current is sync)

**Tasks:**
- [ ] Add event queue struct
- [ ] Implement async processor
- [ ] Add batch writing
- [ ] Implement flush triggers (size, time, session end)
- [ ] Add sync override for specific event types
- [ ] Update collector to use queue

**Files to modify:**
- `trace/collector.rs`
- `trace/mod.rs`
- New: `trace/queue.rs`
- New: `trace/async_processor.rs`

---

### 2.2 Caching Layer

**Status:** Not implemented

**Tasks:**
- [ ] Design cache struct
- [ ] Implement context block cache
- [ ] Implement policy decision cache
- [ ] Add TTL support
- [ ] Add cache invalidation
- [ ] Integrate with resolver

**Files to create:**
- `cache/mod.rs`
- `cache/context_cache.rs`
- `cache/policy_cache.rs`

---

### 2.3 Checkpoint Evaluation

**Status:** Not implemented

**Tasks:**
- [ ] Parse checkpoint config from atlas
- [ ] Implement trigger detection
  - [ ] Keyword matching
  - [ ] Risk tier detection
  - [ ] Time/count intervals
- [ ] Integrate with resolver
- [ ] Add checkpoint events to TRACE

**Files to modify:**
- `carp/resolver.rs`
- New: `carp/checkpoint.rs`

---

### 2.4 Steward Configuration

**Status:** Not implemented

**Tasks:**
- [ ] Add steward structs to manifest
- [ ] Parse steward config from atlas
- [ ] Implement access control checks
- [ ] Implement delivery mode logic
- [ ] Add alert generation

**Files to modify:**
- `atlas/manifest.rs`
- `atlas/loader.rs`
- New: `atlas/steward.rs`

---

### 2.5 Alert System

**Status:** Not implemented

**Tasks:**
- [ ] Define alert event types
- [ ] Implement alert triggers
- [ ] Add webhook support
- [ ] Integrate with TRACE events

**Files to create:**
- `alerts/mod.rs`
- `alerts/triggers.rs`
- `alerts/webhook.rs`

---

## Phase 3: MCP Server Implementation

### 3.1 MCP Server Skeleton

**Status:** Not started

**Tasks:**
- [ ] Set up MCP server project (TypeScript or Rust)
- [ ] Implement tool registration
- [ ] Connect to CRA-Core library
- [ ] Basic request/response flow

**Deliverable:** `cra-mcp/` directory

---

### 3.2 Bootstrap Tool

**Status:** Not started

**Tasks:**
- [ ] Implement `cra_bootstrap` tool
- [ ] Auto-detection logic
- [ ] Discovery questions (minimal)
- [ ] Wrapper template generation
- [ ] Verification flow

---

### 3.3 Context Tools

**Status:** Not started

**Tasks:**
- [ ] Implement `cra_request_context`
- [ ] Implement `cra_list_atlases`
- [ ] Implement `cra_search_contexts`
- [ ] Cache integration

---

### 3.4 Action/Trace Tools

**Status:** Not started

**Tasks:**
- [ ] Implement `cra_report_action`
- [ ] Implement `cra_feedback`
- [ ] Implement `cra_end_session`
- [ ] Async queue integration

---

### 3.5 Help Tools

**Status:** Not started

**Tasks:**
- [ ] Implement `cra_help`
- [ ] Implement `cra_explain`
- [ ] Implement `cra_troubleshoot`
- [ ] Help content database

---

## Phase 4: Wrapper Templates

### 4.1 Claude Code Wrapper

**Status:** Not started

**Tasks:**
- [ ] Design hook structure
- [ ] Implement pre/post tool hooks
- [ ] Context injection point
- [ ] TRACE collection
- [ ] Integration MD generation

**Deliverable:** `wrappers/claude-code/`

---

### 4.2 Generic API Wrapper

**Status:** Not started

**Tasks:**
- [ ] Design middleware structure
- [ ] Request interception
- [ ] Response capture
- [ ] Context injection
- [ ] TRACE collection

**Deliverable:** `wrappers/api-middleware/`

---

### 4.3 Python SDK Wrapper

**Status:** Not started

**Tasks:**
- [ ] Decorator-based wrapper
- [ ] LangChain integration
- [ ] OpenAI client integration
- [ ] Anthropic client integration

**Deliverable:** `wrappers/python-sdk/`

---

## Phase 5: Testing

### 5.1 Unit Tests (Rust Core)

**Status:** Partial (some exist)

**Tasks:**
- [ ] TRACE async queue tests
- [ ] Cache layer tests
- [ ] Checkpoint evaluation tests
- [ ] Steward config tests
- [ ] Alert trigger tests

---

### 5.2 Integration Tests

**Status:** Not started

**Tasks:**
- [ ] End-to-end CARP resolution
- [ ] End-to-end TRACE recording
- [ ] MCP tool integration
- [ ] Wrapper → MCP → Core flow

---

### 5.3 Conformance Tests

**Status:** Partial (some exist)

**Tasks:**
- [ ] Hash chain verification
- [ ] Event ordering
- [ ] Cache invalidation
- [ ] Policy enforcement

---

### 5.4 Agent Tests

**Status:** Not started (previous attempts were simulations)

**Tasks:**
- [ ] Real Claude Code agent test
- [ ] With/without CRA comparison
- [ ] Context injection verification
- [ ] TRACE completeness check
- [ ] Performance benchmarks

---

## Phase 6: Documentation & Examples

### 6.1 Getting Started Guide

**Status:** Not started

**Tasks:**
- [ ] Quick start for developers
- [ ] First atlas creation
- [ ] First wrapper setup
- [ ] Viewing TRACE output

---

### 6.2 Atlas Creation Guide

**Status:** Not started

**Tasks:**
- [ ] Context block writing
- [ ] Policy definition
- [ ] Steward configuration
- [ ] Testing your atlas

---

### 6.3 Example Atlases

**Status:** Partial (VIB3 exists)

**Tasks:**
- [ ] Simple "hello world" atlas
- [ ] API documentation atlas
- [ ] Security policy atlas
- [ ] Multi-tier atlas example

---

## Timeline / Priority

### Immediate (Design Complete)

1. Wrapper Protocol Specification
2. Checkpoint System Specification
3. TRACE Event Types
4. Atlas Schema v2

### Short-term (Core Implementation)

1. Async TRACE
2. Caching Layer
3. Checkpoint Evaluation
4. Steward Configuration

### Medium-term (MCP + Wrappers)

1. MCP Server Skeleton
2. Bootstrap Tool
3. Claude Code Wrapper
4. Context/Action Tools

### Longer-term (Polish)

1. Additional wrapper templates
2. Comprehensive testing
3. Documentation
4. Examples

---

## Dependencies

```
Design Docs ──► Rust Core ──► MCP Server ──► Wrappers ──► Testing
                   │              │
                   └──────────────┴──► Documentation
```

- **Rust Core** depends on design docs being complete
- **MCP Server** depends on Rust Core having async TRACE + cache
- **Wrappers** depend on MCP Server tools existing
- **Testing** depends on wrappers existing
- **Documentation** can proceed in parallel after design

---

## Open Questions

1. **MCP Server Language:** TypeScript (easier MCP ecosystem) or Rust (same as core)?

2. **Wrapper Distribution:** NPM package? Cargo crate? Both?

3. **Atlas Registry:** Local files only? Or also remote registry?

4. **Authentication:** Start with local-only (no auth) or build auth from start?

5. **Cloud Component:** Any hosted component, or purely local/self-hosted?

---

## Success Criteria

### MVP (Minimum Viable Product)

- [ ] Agent can bootstrap and build wrapper
- [ ] Context injection works at checkpoints
- [ ] TRACE records events (async)
- [ ] Basic atlas loading works
- [ ] One wrapper template works (Claude Code)

### V1.0

- [ ] All MCP tools implemented
- [ ] Multiple wrapper templates
- [ ] Steward configuration works
- [ ] Alerts/webhooks work
- [ ] Comprehensive tests pass
- [ ] Documentation complete

### Future

- [ ] Atlas marketplace
- [ ] Hosted CRA option
- [ ] Multi-agent orchestration support
- [ ] Advanced analytics on TRACE data
