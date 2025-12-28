# CRA Architecture & Repository Plan

## Components
- **CRA Runtime**: Resolver and policy engine that evaluates CARP requests, produces CARP responses, and emits TRACE events. Provides in-memory reference runtime and pluggable adapters.
- **TRACE Collector/Store**: JSONL emitter and file-backed store under `traces/`, supporting tail and replay. Hash-links artifacts for audit.
- **CLI (Telemetry Terminal)**: Typer-based CLI that scaffolds projects, issues CARP resolve calls, and tails TRACE streams with filters.
- **Atlas Loader/Validator**: Reads `atlas.json`, validates schema/version, and loads context/policy artifacts.
- **Context Registry**: Persists context issuance/expiry and content hashes under `lock/` to enforce TTL + redaction, emit artifact TRACE events, and support operator-driven refresh of blocks.
- **Registry/Licensing/Certification Service (planned)**: Validates Atlas signatures, enforces licensing, and runs conformance suites.

## Data Models
- **CARP Envelope**: request/response with session, atlas, payload, trace linkage.
- **Resolution Bundle**: context blocks (TTL + redaction), allowed actions with adapters/constraints, deny list, next steps.
- **TRACE Event Schema**: event type, severity, actor, artifacts, span hierarchy; emitted as JSONL.
- **Artifact Model**: name, URI, SHA-256, content type; linked from TRACE and stored locally.
- **Atlas Manifest**: versioned metadata, capabilities, platform adapters, licensing model, CARP/TRACE minimums.

## Extension Points
- **Adapters**: vendor-specific action schemas (OpenAI tools, Anthropic skills, Google ADK, MCP resources). Resolved actions reference adapter ids.
- **Policies**: pluggable allow/deny rules, approval gates, rate limits, redaction rules.
- **Telemetry Sinks**: file, stdout, HTTP, or message bus; TRACE format stable across sinks.
- **Identity**: session principals and scopes can bind to OAuth/JWT/SPIFFE providers.

## Scaling Path
- **Single-node**: in-memory runtime + file TRACE store (current repo).
- **Multi-tenant SaaS**: split runtime, TRACE collector, registry/licensing; introduce durable store (object storage) and message bus.
- **Airgapped enterprise**: offline Atlas install, signed artifacts, local approvals and rate limits.
- **Edge/robotics**: minimal runtime footprint, on-device TRACE cache with periodic uploads.

## Repository Layout
- `src/cra/` runtime, telemetry, models, CLI
- `docs/` protocol and architecture references
- `atlas/reference/` example Atlas
- `templates/` starter agent and prompt files
- `config/`, `lock/`, `traces/` maintained by CLI

## MVP Milestones
1. Protocol docs and architecture plan (this folder)
2. In-memory runtime + TRACE emitter + CARP models
3. CLI commands: `init`, `resolve`, `tail`, `export-trace`
4. Reference Atlas with adapters and context block
5. Conformance harness: structural validation of CARP/TRACE envelopes, golden trace replay stub, adapter scope/approval fixtures under `traces/fixtures/`
