# CRA Roadmap

## Vision

CRA will become the standard context authority layer for agentic systems, enabling:
- **Reliable AI operations** through governed context and explicit permissions
- **Auditable AI behavior** through immutable telemetry
- **Marketplace for expertise** through licensable Atlas packages

## Release Timeline

### v0.1 (Current) - Foundation

**Status: MVP Complete**

Core protocols and local runtime:
- [x] CARP/1.0 specification
- [x] TRACE/1.0 specification
- [x] Local runtime with CARP resolution
- [x] CLI with telemetry terminal
- [x] Atlas loader and validator
- [x] Reference Atlas (GitHub Operations)
- [x] Platform adapters (OpenAI, Claude, MCP)
- [x] Basic conformance testing

**Deployment:** Single-node, CLI-driven

---

### v0.2 - Developer Experience

**Status: Complete**

Improved tooling and integrations:
- [x] HTTP/WebSocket server (`@cra/server`)
- [x] Trace visualization dashboard (`@cra/ui`)
- [x] Golden trace testing framework (`@cra/trace`)
- [x] Dual-mode UI (human terminal + agent JSON API)
- [x] Storage layer with PostgreSQL support (`@cra/storage`)
- [x] Redaction engine for sensitive data (`@cra/trace`)
- [x] Streaming resolution via SSE
- [x] Batch operations endpoint
- [x] OpenTelemetry export (`@cra/otel`)
- [x] MCP server integration (`@cra/mcp`)
- [x] 298 comprehensive tests across 11 packages
- [x] Atlas scaffolding CLI (`cra atlas create`)
- [ ] VS Code extension for Atlas development
- [ ] Interactive policy builder
- [ ] Hot reload for Atlas development

**New Atlases:**
- [ ] AWS Operations
- [ ] Kubernetes Operations
- [ ] Database Queries (SQL/NoSQL)

---

### v0.3 - Protocol Extensions

**Target: Q2 2025**

Enhanced protocol capabilities:
- [x] CARP streaming resolution (SSE-based, implemented in v0.2)
- [ ] TRACE event aggregation and sampling
- [ ] Multi-resolution chaining (subtasks)
- [ ] Conditional action permissions
- [ ] Dynamic policy evaluation

**Runtime:**
- [x] HTTP API server mode (implemented in v0.2)
- [x] WebSocket streaming for TRACE (implemented in v0.2)
- [x] Resolution caching layer (file + PostgreSQL, implemented in v0.2)
- [ ] Metrics export (Prometheus)
- [ ] Redis caching option

---

### v0.4 - Multi-Tenant

**Target: Q3 2025**

Cloud-ready deployment:
- [ ] Multi-tenant isolation
- [ ] API key management
- [ ] Usage metering and quotas
- [ ] Audit logging integration
- [ ] RBAC for Atlas access

**Infrastructure:**
- [ ] Docker/Kubernetes deployment
- [ ] TimescaleDB for TRACE storage
- [ ] S3/GCS for artifact storage
- [ ] Terraform modules

---

### v0.5 - Marketplace Beta

**Target: Q4 2025**

Atlas distribution platform:
- [ ] Atlas registry (publish/discover)
- [ ] Version management (SemVer)
- [ ] License types (free/paid/subscription)
- [ ] Publisher verification
- [ ] Dependency resolution

**Governance:**
- [ ] Certification program for Atlases
- [ ] Automated conformance testing
- [ ] Security scanning

---

### v0.6 - Enterprise Features

**Target: Q1 2026**

Enterprise-grade capabilities:
- [ ] SSO/LDAP integration
- [ ] Custom policy engines
- [ ] SIEM integration for TRACE
- [ ] Compliance reporting (SOC2, HIPAA)
- [ ] Air-gapped deployment

**Privacy:**
- [x] PII detection and redaction (implemented in v0.2 - RedactionEngine)
- [ ] Data residency controls
- [ ] Encryption at rest

---

### v0.7 - Advanced Adapters

**Target: Q2 2026**

Expanded platform support:
- [ ] Google ADK full integration
- [ ] AWS Bedrock adapter
- [ ] LangChain integration
- [ ] AutoGPT integration
- [ ] Custom adapter SDK

**Execution:**
- [ ] Sandboxed action execution
- [ ] Action rollback support
- [ ] Dry-run mode

---

### v0.8 - Edge & Embedded

**Target: Q3 2026**

Resource-constrained environments:
- [ ] Minimal runtime footprint (<10MB)
- [ ] Offline-first operation
- [ ] Sync-when-connected
- [ ] Pre-compiled Atlas bundles
- [ ] Bounded TRACE buffer

**Targets:**
- [ ] Raspberry Pi
- [ ] Industrial controllers
- [ ] Robotics platforms

---

### v0.9 - AI-Native Features

**Target: Q4 2026**

Intelligence layer:
- [ ] Context relevance scoring (ML)
- [ ] Automatic policy suggestions
- [ ] Anomaly detection in TRACE
- [ ] Action outcome prediction
- [ ] Feedback loop integration

---

### v1.0 - General Availability

**Target: Q1 2027**

Production-ready platform:
- [ ] Stable CARP/2.0 specification
- [ ] Stable TRACE/2.0 specification
- [ ] Long-term support (LTS)
- [ ] Comprehensive documentation
- [ ] Certification program
- [ ] Partner ecosystem

---

## Protocol Stability

### Breaking Changes

| Version | CARP | TRACE | Breaking? |
|---------|------|-------|-----------|
| 0.1 → 0.2 | 1.0 | 1.0 | No |
| 0.2 → 0.3 | 1.1 | 1.0 | Minor |
| 0.3 → 0.5 | 1.1 | 1.1 | Minor |
| 0.5 → 1.0 | 2.0 | 2.0 | Major |

### Compatibility Guarantees

- **Patch versions (0.x.Y):** Fully backward compatible
- **Minor versions (0.X.0):** Backward compatible with deprecation warnings
- **Major versions (X.0.0):** May break compatibility; migration guides provided

---

## Community Roadmap

### Contribution Areas

1. **Atlas Development** - Create atlases for new domains
2. **Adapter Development** - Add support for new platforms
3. **Documentation** - Improve guides and examples
4. **Testing** - Expand conformance test coverage
5. **Localization** - Translate documentation

### Governance

- **RFC Process** - Propose protocol changes via RFCs
- **Community Atlases** - Curated collection of community contributions
- **Working Groups** - Focused teams for major features

---

## Metrics for Success

### v0.5 (Marketplace Beta)
- 50+ published Atlases
- 1,000+ active developers
- 3+ enterprise pilots

### v1.0 (GA)
- 500+ published Atlases
- 10,000+ active developers
- 50+ enterprise customers
- 5+ certified partners

---

## How to Contribute

1. **Report Issues** - GitHub Issues for bugs and features
2. **Submit PRs** - Follow contribution guidelines
3. **Write Atlases** - Publish to the marketplace
4. **Join Discord** - Community discussion
5. **Sponsor** - Support development

See [CONTRIBUTING.md](../CONTRIBUTING.md) for details.
