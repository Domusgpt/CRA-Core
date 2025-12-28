# CRA Platform — Executive Brief

## Problem
LLMs routinely misuse proprietary tools, invent APIs and workflows, and provide no proof of execution. This breaks trust, compliance, and scale.

## Solution
Context Registry Agents (CRAs) act as authoritative intermediaries between LLMs and real systems. They curate minimal, correct context; enforce governance and permissions; and emit runtime‑truth telemetry.

## Two Core Protocols
### CARP — Context & Action Resolution Protocol
Determines what context may be injected, what actions are allowed, under what constraints, and with what evidence.

### TRACE — Telemetry & Replay Contract
Defines append‑only telemetry emitted by the runtime that supports replay, diff, and audit‑grade evidence.

## Platform Pillars
- **Runtime authority:** all tool calls flow through the CRA runtime
- **Governance:** authz scopes, approvals, rate limits, redaction
- **Marketplace:** Atlases can be free or licensed; certification ensures CARP/TRACE compliance
- **Developer ergonomics:** CLI‑first, JSONL telemetry, adapter model for multiple vendors

## Outcomes
- Reduced hallucinations and misuse of systems
- Auditability through immutable TRACE streams
- Repeatable deployments via governed Atlases
