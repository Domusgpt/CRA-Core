# CRA × MINOOTS Synergy Analysis

**Date:** 2025-12-28
**Analyst:** Claude (Opus 4.5)
**Status:** Research / Proposal

---

## Executive Summary

**CRA** (Context Registry Agents) and **MINOOTS** (Timer System) are highly complementary systems that together could form a complete **Agent Control Plane**:

| System | Core Function | Question Answered |
|--------|---------------|-------------------|
| **CRA** | Governance & Audit | *What* can agents do? *What* did they do? |
| **MINOOTS** | Durable Scheduling | *When* should agents act? |

**Recommendation:** Strong synergy potential. Integration would create a unified agent orchestration layer with governance, auditing, and temporal control.

---

## System Comparison

### CRA (Context Registry Agents)

```
┌─────────────────────────────────────────────┐
│                    CRA                       │
├─────────────────────────────────────────────┤
│  CARP: Context & Action Resolution Protocol │
│  - What actions are allowed?                │
│  - What context is available?               │
│  - Policy enforcement (deny, rate-limit)    │
├─────────────────────────────────────────────┤
│  TRACE: Telemetry & Replay Audit            │
│  - Cryptographic event log                  │
│  - Hash-chain integrity                     │
│  - Behavioral replay/diff                   │
├─────────────────────────────────────────────┤
│  Atlas: Governance Packages                 │
│  - Versioned capability bundles             │
│  - Domain-scoped policies                   │
│  - Portable agent configurations            │
└─────────────────────────────────────────────┘
```

### MINOOTS (Timer System)

```
┌─────────────────────────────────────────────┐
│                  MINOOTS                     │
├─────────────────────────────────────────────┤
│  Control Plane: REST API                    │
│  - Timer CRUD operations                    │
│  - Multi-tenant support                     │
│  - Team collaboration                       │
├─────────────────────────────────────────────┤
│  Horology Kernel: Rust/Tokio Scheduler      │
│  - Independent timer execution              │
│  - Survives crashes/reboots                 │
│  - Event broadcasting                       │
├─────────────────────────────────────────────┤
│  Action Orchestrator: Event Consumer        │
│  - Webhook triggers                         │
│  - CLI command execution                    │
│  - File write operations                    │
└─────────────────────────────────────────────┘
```

---

## Synergy Opportunities

### 1. **Governed Timer Actions** (High Value)

**Problem:** MINOOTS timers can trigger any webhook/command without governance checks.

**Solution:** Route timer actions through CRA for policy enforcement.

```
Current:
  Timer expires → Action Orchestrator → Execute webhook directly

With CRA:
  Timer expires → Action Orchestrator → CRA.resolve() → Execute if allowed
                                              ↓
                                        TRACE audit log
```

**Benefits:**
- Timer actions respect the same policies as real-time agent actions
- Complete audit trail even for scheduled/deferred work
- Rate limiting applies to timer-triggered actions

### 2. **Temporal Policies in Atlas** (High Value)

**Problem:** CRA policies are stateless - no concept of time-based rules.

**Solution:** Integrate MINOOTS timing into Atlas policy definitions.

```yaml
# Example: Atlas policy with temporal constraints
policies:
  - policy_id: "business-hours-only"
    type: "temporal"
    actions: ["email.send", "slack.post"]
    schedule:
      allowed_hours: "09:00-17:00"
      timezone: "America/New_York"
      days: ["mon", "tue", "wed", "thu", "fri"]

  - policy_id: "cooldown-period"
    type: "temporal"
    actions: ["deployment.trigger"]
    cooldown:
      duration: "30m"
      scope: "per_session"  # or per_tenant, global
```

**Benefits:**
- Time-aware governance without custom code
- Business hours enforcement
- Cooldown periods between sensitive actions

### 3. **Scheduled TRACE Exports** (Medium Value)

**Problem:** TRACE logs need periodic export for compliance/archival.

**Solution:** Use MINOOTS to schedule TRACE data exports.

```
MINOOTS Timer (daily at 2am)
    → Trigger CRA TRACE export
    → Write to S3/GCS
    → Emit completion event
```

### 4. **Agent Task Timeouts with Audit** (High Value)

**Problem:** Long-running agent tasks need timeouts, but timeout events should be audited.

**Solution:** Unified timeout management.

```rust
// Agent starts task
let session = cra.create_session(agent_id, goal);
let timeout_timer = minoots.create_timer(
    name: format!("timeout:{}", session.id),
    duration: "30m",
    action: webhook(cra.timeout_endpoint(session.id)),
    metadata: { session_id, reason: "task_timeout" }
);

// If agent completes first
cra.end_session(session.id);
minoots.cancel_timer(timeout_timer.id);

// If timeout fires first
// → CRA records SessionTimeout event in TRACE
// → Session marked as timed out
// → Audit trail preserved
```

### 5. **Shared Rust Core** (Technical Synergy)

Both systems have Rust components:
- CRA: `cra-core` (CARP, TRACE, Atlas)
- MINOOTS: `horology-kernel` (Tokio scheduler)

**Opportunity:** Shared crate for:
- Common error types
- Unified observability (tracing/metrics)
- Shared tenant/session models

```
┌─────────────────────────────────────────────┐
│            agent-platform-core               │
├─────────────────────────────────────────────┤
│  - TenantId, SessionId, AgentId types       │
│  - Common error handling                    │
│  - Tracing instrumentation                  │
│  - Health check traits                      │
└─────────────────────────────────────────────┘
         ↑                    ↑
    cra-core            horology-kernel
```

### 6. **Unified Agent SDK** (High Value)

**Current state:**
- CRA has: `cra-python` (PyO3 bindings)
- MINOOTS has: `minoots_agent_tools` (Python client)

**Opportunity:** Single SDK for agent developers.

```python
from agent_platform import AgentRuntime, Timer

# Unified interface
runtime = AgentRuntime(
    cra_endpoint="https://cra.example.com",
    minoots_endpoint="https://minoots.example.com",
    api_key="...",
)

# Governed action
result = runtime.execute(
    action="email.send",
    parameters={"to": "user@example.com", "subject": "Hello"},
)  # → CRA policy check → execution → TRACE log

# Scheduled governed action
runtime.schedule(
    action="report.generate",
    parameters={"type": "weekly"},
    delay="1h",
)  # → MINOOTS timer → CRA policy check on trigger
```

---

## Integration Architecture

### Option A: Loose Coupling (Recommended Start)

```
┌─────────────────┐     ┌─────────────────┐
│   MINOOTS       │     │      CRA        │
│  Control Plane  │────▶│    cra-server   │
│                 │ HTTP│                 │
└─────────────────┘     └─────────────────┘
        │                       │
        ▼                       ▼
┌─────────────────┐     ┌─────────────────┐
│   Horology      │     │    cra-core     │
│    Kernel       │     │   (Resolver)    │
└─────────────────┘     └─────────────────┘
```

- MINOOTS action orchestrator calls CRA HTTP API before executing
- Minimal code changes
- Systems remain independently deployable

### Option B: Tight Coupling (Future)

```
┌─────────────────────────────────────────────┐
│           Agent Platform Runtime             │
├─────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────────────┐   │
│  │  cra-core   │  │  horology-kernel    │   │
│  │  (embedded) │  │    (embedded)       │   │
│  └─────────────┘  └─────────────────────┘   │
│            ↓              ↓                 │
│  ┌──────────────────────────────────────┐   │
│  │         Shared Event Bus              │   │
│  │   (timers, policies, traces)          │   │
│  └──────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
```

- Single binary deployment
- In-process communication
- Shared state management

---

## Implementation Roadmap

### Phase 1: Webhook Integration (1-2 weeks)
- [ ] MINOOTS action orchestrator calls CRA `/v1/resolve` before webhooks
- [ ] Add `cra_session_id` to timer metadata
- [ ] TRACE events for timer-triggered actions

### Phase 2: Temporal Policies (2-3 weeks)
- [ ] Add `temporal` policy type to Atlas schema
- [ ] Implement schedule/cooldown evaluation in cra-core
- [ ] MINOOTS provides current time context to CRA

### Phase 3: Unified SDK (2-3 weeks)
- [ ] Create `agent-platform` Python package
- [ ] Wrap both CRA and MINOOTS clients
- [ ] Unified configuration and authentication

### Phase 4: Shared Rust Core (3-4 weeks)
- [ ] Extract common types to `agent-platform-core` crate
- [ ] Both systems depend on shared crate
- [ ] Unified error handling and observability

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| API versioning conflicts | Medium | High | Semantic versioning, deprecation policy |
| Performance overhead | Low | Medium | Async CRA calls, caching |
| Deployment complexity | Medium | Medium | Start with loose coupling |
| Tenant isolation | Low | High | Enforce tenant context in both systems |

---

## Conclusion

**Strong synergy exists.** CRA and MINOOTS solve orthogonal problems (governance vs. timing) for the same user base (autonomous agents). Integration would create a more complete agent platform without significant architectural conflicts.

**Recommended first step:** Implement Phase 1 (webhook integration) to validate the value proposition with minimal investment.

---

## References

- CRA: This repository
- MINOOTS: https://github.com/Domusgpt/minoots-timer-system
- Branch: `codex/16-21-14complete-phase-3-and-phase-2-work2025-10-29`
