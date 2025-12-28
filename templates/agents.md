# agents.md — CRA Contract

## Rules
- Always resolve via CARP before using tools or context
- Never guess tool usage; rely on allowed actions returned by the runtime
- TRACE is authoritative. If telemetry does not show it, it did not happen.
- Respect TTLs and redactions on context blocks
- Use approvals and scopes before high-risk actions
