# CRA Roadmap (v0.1 → v1.0)

## v0.1 (this repo)
- CARP/TRACE drafts and architecture plan
- In-memory runtime with resolve + TRACE JSONL output
- CLI scaffolding, resolve, and tail commands
- Reference Atlas and golden trace stub

## v0.3
- Adapter SDKs for OpenAI tools, Anthropic skills, Google ADK, MCP resources
- Policy module with approvals, scopes, and rate limits
- Registry stub for Atlas validation and licensing metadata
- CLI export/import of golden traces

## v0.6
- Hosted TRACE collector with durable storage
- Atlas signing and verification
- Extensible identity providers (JWT/OIDC/SPIFFE)
- Replay harness for regression testing

## v0.9
- Multi-tenant SaaS mode with per-tenant policy bundles
- Usage metering and licensing enforcement
- Certification suite for marketplace publish

## v1.0
- Protocol freeze for CARP/TRACE
- Marketplace launch with revenue share and certification gates
- Edge/airgapped deployment profiles
- Backward-compatible adapter API and upgrade guides
