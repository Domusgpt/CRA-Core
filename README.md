# CRA-Core
Contest registry agent CARP/TRACE
# CRA Spec Pack v0.1

> This document contains the **complete, clean, copy‑safe versions** of all CRA specifications and onboarding files. You can **Save As → Markdown**, split into files, or paste into your repo exactly as‑is. No broken formatting. No hidden blocks. No missing sections.

---

## TABLE OF CONTENTS

1. ONE\_PAGER.md
2. EXECUTIVE\_BRIEF.md
3. CARP\_1\_0.md
4. TRACE\_1\_0.md
5. RUNTIME\_API\_v0\_1.md
6. ATLAS\_FORMAT\_v0\_1.md
7. CLI\_TELEMETRY\_REQUIREMENTS\_v0\_1.md
8. templates/agents.md
9. templates/CRA\_STARTER\_PROMPT.md

---

# ==========================

# 1. ONE\_PAGER.md

# ==========================

# CRA — Context Registry Agents

**What**\
A governed context layer that makes AI agents use tools, systems, and proprietary knowledge *correctly*.

**How**\
CARP resolves context + permitted actions. TRACE proves what happened.

**Why**\
LLMs hallucinate. Enterprises need auditability. Creators need a way to publish operational knowledge.

---

## Core Components

- **CRA Runtime** – authoritative resolver + policy engine
- **CARP** – Context & Action Resolution Protocol (meaning)
- **TRACE** – Telemetry & Replay Contract (truth)
- **Atlases** – creator‑published context packages
- **CLI** – asynchronous, telemetry‑first UX

---

## What Makes This Unique

1. Context is treated as a **licensed, governed artifact**
2. Runtime is authoritative — LLM narration is non‑authoritative
3. Telemetry is replayable, diffable, and audit‑grade
4. One Atlas works across Claude, OpenAI, Google ADK, MCP

---

## Business Model

Creators publish Atlases:

- free, paid, subscription, usage‑metered
- platform takes a revenue cut
- certification for CARP/TRACE compliance

---

## One‑Line Positioning

**“The context authority layer that makes agents reliable, governable, and provable.”**

---

# ==========================

# 2. EXECUTIVE\_BRIEF.md

# ==========================

# CRA Platform — Executive Brief

## Problem

LLMs routinely:

- misuse proprietary tools
- invent APIs and workflows
- provide no proof of execution

This breaks trust, compliance, and scale.

---

## Solution

**Context Registry Agents (CRAs)** act as *authoritative intermediaries* between LLMs and real systems.

They:

- curate minimal, correct context
- enforce governance and permissions
- emit runtime‑truth telemetry

---

## Two Core Protocols

### CARP — Context & Action Resolution Protocol

A strict contract that decides:

- what context may be injected
- what actions are allowed
- under what constraints
- with what evidence

### TRACE — Telemetry & Replay Contract

A runtime‑emitted flight recorder:

- deterministic replay
- regression testing
- compliance audits

---

## Market Impact

- Enterprises: safe agentic automation
- Developers: correct tool usage
- Creators: monetize operational expertise

---

# ==========================

# 3. CARP\_1\_0.md

# ==========================

# CARP/1.0 — Context & Action Resolution Protocol

## Purpose

CARP defines a deterministic contract between:

- an **acting agent**, and
- a **context authority (CRA)**

CARP answers *what context is allowed* and *what actions may occur*.

---

## Core Guarantees

- Runtime authority (not the model)
- Least‑privilege context
- Explicit permissions
- Evidence‑backed decisions

---

## CARP Request (Example)

```json
{
  "carp_version": "1.0",
  "operation": "resolve",
  "task": {
    "goal": "Deploy internal service",
    "risk_tier": "high"
  }
}
```

---

## CARP Resolution (Example)

```json
{
  "resolution": {
    "decision": "allow_with_constraints",
    "context_blocks": ["deployment_rules"],
    "allowed_actions": ["ci.deploy"],
    "requires_approval": true
  }
}
```

---

# ==========================

# 4. TRACE\_1\_0.md

# ==========================

# TRACE/1.0 — Telemetry & Replay Contract

## Principle

> If it wasn’t emitted by the runtime, it didn’t happen.

---

## What TRACE Is

- append‑only event stream
- emitted by CRA runtime
- supports replay and diff

---

## TRACE Event (Example)

```json
{
  "trace_version": "1.0",
  "event_type": "carp.resolve",
  "timestamp": "2025‑01‑01T12:00:00Z"
}
```

---

# ==========================

# 5. RUNTIME\_API\_v0\_1.md

# ==========================

# CRA Runtime API v0.1

## Key Endpoints

- POST /carp/resolve
- POST /carp/execute
- GET  /trace/stream
- GET  /trace/replay

---

## Runtime Rules

- All tool calls go through runtime
- All telemetry is runtime‑emitted
- CLI is the primary interface

---

# ==========================

# 6. ATLAS\_FORMAT\_v0\_1.md

# ==========================

# CRA Atlas Format

Atlases are creator‑published packages that include:

- atlas.json (manifest)
- context packs
- policies
- adapters (Claude/OpenAI/MCP)
- tests

---

# ==========================

# 7. CLI\_TELEMETRY\_REQUIREMENTS\_v0\_1.md

# ==========================

# CLI Telemetry Requirements

- asynchronous by default
- continuous TRACE output
- JSON‑first, human‑friendly second

---

# ==========================

# 8. templates/agents.md

# ==========================

# agents.md — CRA Contract

## Rules

- Always resolve via CARP
- Never guess tool usage
- TRACE is authoritative

---

# ==========================

# 9. templates/CRA\_STARTER\_PROMPT.md

# ==========================

# CRA Starter Prompt

You operate under CRA governance.

Rules:

- Ask CARP before acting
- Use only allowed actions
- Rely on TRACE, not narration

---

