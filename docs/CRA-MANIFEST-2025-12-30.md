# CRA Complete System Manifest
**Document ID:** CRA-MANIFEST-2025-12-30
**Version:** 1.0.0
**Last Audit:** 2025-12-30
**Status:** Active Development

---

# Table of Contents

1. [Executive Summary](#executive-summary)
2. [System Architecture](#system-architecture)
3. [Protocol Specifications](#protocol-specifications)
4. [Atlas System](#atlas-system)
5. [Implementation Details](#implementation-details)
6. [Testing Framework](#testing-framework)
7. [Documentation Systems](#documentation-systems)
8. [Current State](#current-state)
9. [Development Roadmap](#development-roadmap)
10. [Appendices](#appendices)

---

# Executive Summary

## What is CRA?

CRA (Context Registry for Agents) is a **governance and audit infrastructure** for AI agents. It solves two fundamental problems:

1. **Governance:** How do we control what AI agents can do?
2. **Accountability:** How do we prove what AI agents did?

## The Three Pillars

| Protocol | Purpose | Analogy |
|----------|---------|---------|
| **CARP** | Policy + Context Resolution | "What can I do? What should I know?" |
| **TRACE** | Cryptographic Audit Trail | "Prove what happened" |
| **Atlas** | Domain Knowledge Packages | "Here's everything about X" |

## Core Philosophy

> **"If it wasn't emitted by the runtime, it didn't happen."**

Traditional AI systems rely on the LLM to report what it did. CRA inverts this: the runtime authoritatively records every action, creating a tamper-evident audit trail.

## Why This Matters

- **Enterprise Compliance:** Audit trails for regulated industries
- **Multi-Agent Systems:** Governance across agent handoffs
- **High-Stakes Operations:** Proof of correct behavior
- **Debugging:** Replay and analyze exactly what happened

---

# System Architecture

## High-Level Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                         AGENT                                   │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      CRA RESOLVER                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │    CARP      │  │    TRACE     │  │    ATLAS     │          │
│  │  (Policy +   │  │ (Audit Log)  │  │  (Domain     │          │
│  │   Context)   │  │              │  │   Knowledge) │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    CRYPTOGRAPHIC PROOF                          │
│         Hash Chain of All Events (Tamper-Evident)               │
└─────────────────────────────────────────────────────────────────┘
```

## Data Flow

```
1. Agent sends CARPRequest
   └── Contains: session_id, agent_id, goal

2. Resolver processes request
   ├── Emits: carp.request.received
   ├── Evaluates policies for each action
   │   └── Emits: policy.evaluated (per action)
   ├── Matches context blocks to goal
   │   └── Emits: context.injected (per block)
   └── Emits: carp.resolution.completed

3. Resolver returns CARPResolution
   └── Contains: decision, allowed_actions, denied_actions, context_blocks

4. Agent executes action (optional)
   ├── Calls: resolver.execute(action_id, params)
   ├── Emits: action.requested
   ├── Emits: action.approved OR action.denied
   └── Emits: action.executed

5. Audit
   ├── resolver.get_trace(session_id) → All events
   └── resolver.verify_chain(session_id) → Integrity check
```

## Component Relationships

```
                    ┌─────────────────┐
                    │    Resolver     │
                    │   (Main API)    │
                    └────────┬────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         ▼                   ▼                   ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│ PolicyEvaluator │ │ ContextRegistry │ │ TraceCollector  │
│                 │ │                 │ │                 │
│ - Policies      │ │ - Context Blocks│ │ - Events        │
│ - Allow/Deny    │ │ - Matching      │ │ - Hash Chain    │
└─────────────────┘ └─────────────────┘ └─────────────────┘
         │                   │                   │
         └───────────────────┼───────────────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │  AtlasManifest  │
                    │                 │
                    │ - context_blocks│
                    │ - actions       │
                    │ - policies      │
                    └─────────────────┘
```

---

# Protocol Specifications

## CARP Protocol

### Purpose
Resolve what actions are allowed and what context is relevant for a given goal.

### Request Structure
```rust
pub struct CARPRequest {
    pub session_id: String,      // Session this request belongs to
    pub agent_id: String,        // Agent making the request
    pub goal: String,            // Natural language goal
    pub context_hints: Option<Vec<String>>,  // Optional hints for context
    pub action_hints: Option<Vec<String>>,   // Optional hints for actions
}
```

### Resolution Structure
```rust
pub struct CARPResolution {
    pub session_id: String,
    pub trace_id: String,
    pub decision: Decision,           // Allow, Deny, Partial, AllowWithConstraints
    pub allowed_actions: Vec<AllowedAction>,
    pub denied_actions: Vec<DeniedAction>,
    pub constraints: Vec<Constraint>,
    pub context_blocks: Vec<ContextBlock>,  // Injected context
    pub ttl_seconds: u64,
    pub timestamp: DateTime<Utc>,
}
```

### Decision Types
```rust
pub enum Decision {
    Allow,               // All requested actions allowed
    Deny,                // All actions denied
    Partial,             // Some allowed, some denied
    AllowWithConstraints,// Allowed with conditions
}
```

### Policy Evaluation
Policies are evaluated in order. First match wins.

```rust
pub enum PolicyType {
    Allow,           // Explicitly allow
    Deny,            // Explicitly deny
    RequireApproval, // Require human approval
    RateLimit,       // Limit frequency
    Audit,           // Allow but log
}
```

---

## TRACE Protocol

### Purpose
Create cryptographic proof of everything that happened.

### Event Structure
```rust
pub struct TRACEEvent {
    pub trace_version: String,      // "1.0"
    pub event_id: String,           // UUID
    pub trace_id: String,           // Groups related events
    pub span_id: String,            // This operation
    pub parent_span_id: Option<String>,
    pub session_id: String,
    pub sequence: u64,              // Monotonically increasing
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub payload: Value,             // Event-specific data
    pub event_hash: String,         // SHA-256 of this event
    pub previous_event_hash: String,// Links to prior event
}
```

### Hash Computation
**CRITICAL:** Never reimplement. Use `TRACEEvent::compute_hash()`.

```rust
// Hash components in order:
hasher.update(trace_version);
hasher.update(event_id);
hasher.update(trace_id);
hasher.update(span_id);
hasher.update(parent_span_id.unwrap_or(""));
hasher.update(session_id);
hasher.update(sequence.to_string());
hasher.update(timestamp.to_rfc3339());
hasher.update(event_type.as_str());
hasher.update(canonical_json(&payload));  // Sorted keys!
hasher.update(previous_event_hash);
```

### Event Types
```rust
pub enum EventType {
    SessionStarted,          // session.started
    SessionEnded,            // session.ended
    CARPRequestReceived,     // carp.request.received
    CARPResolutionCompleted, // carp.resolution.completed
    PolicyEvaluated,         // policy.evaluated
    ContextInjected,         // context.injected
    ActionRequested,         // action.requested
    ActionApproved,          // action.approved
    ActionDenied,            // action.denied
    ActionExecuted,          // action.executed
    Custom(String),          // Custom events
}
```

### Chain Verification
```rust
pub struct ChainVerification {
    pub is_valid: bool,
    pub event_count: usize,
    pub first_event_id: Option<String>,
    pub last_event_id: Option<String>,
    pub errors: Vec<ChainError>,
}
```

### Deferred Mode
For high-throughput scenarios (<1µs per event instead of ~15µs):

```rust
let resolver = Resolver::new()
    .with_deferred_tracing(DeferredConfig::default());

// Fast operations (no hash computation)
resolver.resolve(&request)?;

// Flush before querying
resolver.flush_traces()?;
let trace = resolver.get_trace(&session_id)?;
```

---

## Atlas Schema

### Purpose
Package domain knowledge for agent governance.

### Manifest Structure
```rust
pub struct AtlasManifest {
    pub atlas_version: String,       // "1.0"
    pub atlas_id: String,            // Unique identifier
    pub version: String,             // Semver
    pub name: String,
    pub description: String,
    pub authors: Vec<String>,
    pub license: Option<String>,
    pub domains: Vec<String>,        // Applicable domains
    pub sources: Option<AtlasSources>,
    pub capabilities: Vec<AtlasCapability>,
    pub context_packs: Vec<AtlasContextPack>,
    pub context_blocks: Vec<AtlasContextBlock>,
    pub policies: Vec<AtlasPolicy>,
    pub actions: Vec<AtlasAction>,
}
```

### Context Block Structure
```rust
pub struct AtlasContextBlock {
    pub context_id: String,
    pub name: String,
    pub priority: i32,               // Higher = more important
    pub inject_mode: InjectMode,
    pub also_inject: Vec<String>,    // Other blocks to include
    pub content: String,             // The actual content (usually markdown)
    pub content_type: String,        // "text/markdown"
    pub inject_when: Vec<String>,    // Condition expressions
    pub keywords: Vec<String>,       // For goal matching
    pub risk_tiers: Vec<String>,     // Risk-based injection
}
```

### Injection Modes
```rust
pub enum InjectMode {
    Always,     // Always inject when atlas is active
    OnMatch,    // When keywords match goal (default)
    OnDemand,   // Only if explicitly requested
    RiskBased,  // Based on request risk tier
}
```

### Sources Structure
```rust
pub struct AtlasSources {
    pub repositories: Vec<String>,
    pub documentation: Option<String>,
    pub demo: Option<String>,
}
```

---

# Atlas System

## Design Philosophy

### Why Atlases?
Instead of hardcoding domain knowledge into the runtime, atlases allow:
1. **Versioning:** Different versions for different needs
2. **Composition:** Load multiple atlases together
3. **Distribution:** Share domain knowledge as packages
4. **Updates:** Update knowledge without changing code

### Content Design Principles

1. **Be Specific:** Don't say "use the API" - show the exact code
2. **Prevent Mistakes:** Document what NOT to do
3. **Enable Discovery:** Tell agents how to find more information
4. **Don't Over-Constrain:** Leave room for experimentation

### Priority Guidelines
| Range | Use For |
|-------|---------|
| 400+ | Essential facts that should always appear |
| 300-399 | Workflow guidance for specific tasks |
| 200-299 | Reference material |
| 100-199 | Debug info, advanced topics |
| 0-99 | Supplementary info |

## Current Atlases

### cra-development.json
**Purpose:** Self-governance for CRA development

**Key blocks:**
- `read-before-modify` (250) - Always read files first
- `hash-computation-rule` (200) - Never reimplement hash
- `chain-invariants` (190) - TRACE chain rules
- `deferred-mode-pattern` (180) - High-throughput tracing
- `module-boundaries` (105) - Code organization

### vib3-webpage-development.json (v6.0.0)
**Purpose:** Context for VIB3+ shader visualization

**Key blocks:**
- `vib3-what-it-is` (400, always) - System overview
- `vib3-geometry-system` (380) - 24 geometries
- `vib3-javascript-api` (360) - API reference
- `vib3-embed-iframe` (350) - Embedding guide
- `vib3-keyboard-shortcuts` (340) - All shortcuts
- `vib3-audio-reactivity` (320) - Audio integration
- `vib3-interactivity` (310) - Mouse/touch
- `vib3-common-recipes` (300) - Code examples
- `vib3-parameters-reference` (200) - Parameter list
- `vib3-state-debugging` (180) - Debug tools

---

# Implementation Details

## Rust Crate Structure

### cra-core
Main library implementing CARP, TRACE, and Atlas.

**Features:**
- `ffi` - C FFI bindings
- `async-runtime` - Async support

**Dependencies:**
- `chrono` - Timestamps
- `uuid` - Event/session IDs
- `sha2` - Hash computation
- `serde`/`serde_json` - Serialization
- `hex` - Hash encoding

### cra-wasm
WASM bindings for browser use (work in progress).

## Key Implementation Notes

### Hash Chain Integrity
The hash chain is THE core security feature. Every event's hash includes:
- All event metadata
- Canonical JSON of payload (sorted keys!)
- Previous event's hash

If any event is modified, all subsequent hashes become invalid.

### Context Matching Algorithm
```
1. For each context block in loaded atlases:
   a. Check inject_mode
      - Always: Include
      - OnMatch: Check keywords
      - OnDemand: Check context_hints
      - RiskBased: Check risk tier

   b. For OnMatch:
      - Tokenize goal
      - Match against block keywords
      - Calculate match score
      - If score > threshold, include

   c. Apply also_inject recursively

2. Sort by priority (descending)
3. Emit context.injected event for each
4. Return in resolution
```

### Session Lifecycle
```
1. create_session(agent_id, goal)
   └── Emits session.started

2. resolve(request) [0..n times]
   └── Emits request, policy, context, resolution events

3. execute(action) [0..n times]
   └── Emits request, approve/deny, executed events

4. end_session(session_id)
   └── Emits session.ended with stats
```

---

# Testing Framework

## Test Categories

### 1. Unit Tests (in src/)
Test individual functions and methods.
```bash
cargo test --lib
```

### 2. Integration Tests (in tests/)
Test complete workflows.
```bash
cargo test --tests
```

### 3. Conformance Tests
Test against golden traces from specs/.
```bash
cargo test --test conformance
```

### 4. Self-Governance Tests
Test CRA governing its own development.
```bash
cargo test --test self_governance
```

## Current Test Status

| Test Category | Status | Notes |
|---------------|--------|-------|
| Unit tests | ✅ Pass | Core functionality |
| Integration flow | ✅ Pass | Full workflows |
| Self-governance | ✅ Pass | Context injection |
| Conformance | ⚠️ Needs fixes | Schema migration |
| VIB3 context demo | ✅ Pass | Atlas testing |

## Missing: Real Agentic Tests

**The Gap:** We have NOT tested with actual AI agents performing tasks.

**What's needed:**
1. Two agents given same task
2. Agent A: No CRA context
3. Agent B: With CRA context injected
4. Same environment, same resources
5. Record and compare:
   - What they tried
   - Errors encountered
   - Time to completion
   - Final result quality

**Why this matters:**
- Unit tests verify mechanics work
- Agentic tests verify the system HELPS
- Without this, we don't know if context actually improves outcomes

---

# Documentation Systems

## Document Types

### 1. Code Documentation (CLAUDE.md)
**Location:** `/CLAUDE.md`
**Purpose:** Instructions for AI agents working on this codebase

**Contains:**
- What the project is
- Key files to read first
- How to run tests
- Current branch information

### 2. Protocol Specifications (specs/)
**Location:** `/specs/`
**Purpose:** Formal protocol definitions

**Contains:**
- Golden traces for conformance testing
- Expected behavior documentation

### 3. Development Guides (docs/)
**Location:** `/docs/`
**Purpose:** Human and AI readable documentation

**Key files:**
- `CRA_TESTING_PLAN.md` - Testing strategy
- `CRA_QUICK_OVERVIEW.md` - Quick reference
- `CRA_LANDSCAPE_ANALYSIS.md` - Comparison to other systems
- `VIB3_COMPLETE_USAGE_GUIDE.md` - VIB3 reference

### 4. Handoff Documents (docs/CRA-DEV-*)
**Location:** `/docs/CRA-DEV-{date}.md`
**Purpose:** Session handoffs between developers/agents

**Contains:**
- Current state summary
- What's been done
- What's not done
- Next steps

### 5. Manifest Documents (docs/CRA-MANIFEST-*)
**Location:** `/docs/CRA-MANIFEST-{date}.md`
**Purpose:** Complete system specification (this document)

**Contains:**
- Full architecture
- All protocols
- Implementation details
- Testing framework
- Roadmap

## Documentation Philosophy

### Why Multiple Document Types?
Different readers need different information:
- **CLAUDE.md:** AI agents need quick, actionable instructions
- **Specs:** Testing needs precise expected behavior
- **Guides:** Humans need conceptual understanding
- **Manifests:** Auditors need complete picture

### Versioning
Documents with dates (CRA-DEV-*, CRA-MANIFEST-*) are snapshots. When the system is audited or significantly updated, create a new dated version rather than modifying the old one.

---

# Current State

## What Works
- ✅ CARP resolution
- ✅ TRACE event emission
- ✅ Hash chain computation
- ✅ Chain verification
- ✅ Atlas loading
- ✅ Context matching
- ✅ Policy evaluation
- ✅ Action execution recording
- ✅ Deferred tracing mode
- ✅ Self-governance atlas
- ✅ VIB3 atlas (verified against source)

## What's Incomplete
- ⚠️ Some test files need schema updates
- ⚠️ InjectMode::Always not fully implemented in resolver
- ⚠️ also_inject not recursively resolved
- ⚠️ WASM build incomplete

## What's Missing
- ❌ Real agentic tests
- ❌ Performance benchmarks with agents
- ❌ Production deployment guide
- ❌ Atlas creation tooling

## Known Issues
1. Test files `conformance.rs` and some helpers need `inject_mode` and `also_inject` fields
2. Some tests have fake "simulation" code that should be removed
3. VIB3 atlas may need updates as VIB3 evolves

---

# Development Roadmap

## Phase 1: Test Stabilization (Current)
- [ ] Fix schema compilation errors in test files
- [ ] Remove simulation tests
- [ ] Verify all 128+ tests pass
- [ ] Document test coverage

## Phase 2: Real Agent Testing
- [ ] Design agent comparison protocol
- [ ] Set up test environment
- [ ] Run Agent A (no CRA) vs Agent B (with CRA)
- [ ] Document results
- [ ] Iterate on atlas content based on findings

## Phase 3: Feature Completion
- [ ] Implement InjectMode::Always in resolver
- [ ] Implement recursive also_inject
- [ ] Complete WASM build
- [ ] Add more atlases

## Phase 4: Production Readiness
- [ ] Performance optimization
- [ ] Security audit
- [ ] Deployment documentation
- [ ] Atlas creation tooling

## Phase 5: Ecosystem
- [ ] Atlas marketplace/registry
- [ ] Integration guides for popular frameworks
- [ ] Monitoring/observability tools

---

# Appendices

## A. File Listing

### Core Source
```
cra-core/src/
├── lib.rs
├── error.rs
├── carp/
│   ├── mod.rs
│   ├── resolver.rs
│   ├── request.rs
│   ├── resolution.rs
│   └── policy.rs
├── trace/
│   ├── mod.rs
│   ├── event.rs
│   ├── collector.rs
│   ├── chain.rs
│   ├── buffer.rs
│   ├── processor.rs
│   ├── replay.rs
│   └── raw.rs
├── atlas/
│   ├── mod.rs
│   ├── manifest.rs
│   ├── loader.rs
│   └── validator.rs
├── context/
│   ├── mod.rs
│   ├── registry.rs
│   └── matcher.rs
├── storage/
│   └── mod.rs
└── timing/
    ├── mod.rs
    ├── manager.rs
    └── backends/
```

### Tests
```
cra-core/tests/
├── conformance.rs
├── context_demo.rs
├── integration_flow.rs
├── self_governance.rs
├── vib3_context_demo.rs
└── vib3_detailed_execution.rs
```

### Atlases
```
atlases/
├── cra-development.json
├── cra-development-v3.json
└── vib3-webpage-development.json
```

### Documentation
```
docs/
├── CRA_TESTING_PLAN.md
├── CRA_QUICK_OVERVIEW.md
├── CRA_LANDSCAPE_ANALYSIS.md
├── VIB3_COMPLETE_USAGE_GUIDE.md
├── VIB3_VERIFICATION_PLAN.md
├── VIB3_VERIFICATION_RESULTS.md
├── CRA-DEV-2025-12-30.md
└── CRA-MANIFEST-2025-12-30.md
```

## B. Quick Commands

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Specific test
cargo test test_name -- --nocapture

# Check compilation
cargo check

# Build release
cargo build --release

# Format code
cargo fmt

# Lint
cargo clippy
```

## C. Common Errors

### "missing field 'inject_mode'"
Test code creating AtlasContextBlock needs the new fields:
```rust
AtlasContextBlock {
    // ... existing fields ...
    inject_mode: InjectMode::OnMatch,
    also_inject: vec![],
}
```

### "hash verification failed"
Check that you're using `TRACEEvent::compute_hash()` and not reimplementing hash logic.

### "context not injected"
Check that:
1. Keywords match goal text
2. inject_mode is appropriate
3. Atlas is loaded before creating session

---

# Document Control

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-12-30 | CRA Dev | Initial manifest |

**Next Review:** When significant changes are made or before major release.
