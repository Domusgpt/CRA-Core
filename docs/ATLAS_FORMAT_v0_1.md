# CRA Atlas Format v0.1

An Atlas is a distributable package of context, policies, adapters, tests, and prompts. It is versioned and licensable.

```
atlas/
  atlas.json
  README.md
  context/
    compact.md
  policies/
  adapters/
  tests/
  prompts/
```

## Minimal `atlas.json`
```json
{
  "atlas_version": "0.1",
  "id": "publisher.slug",
  "name": "Human name",
  "version": "0.1.0",
  "publisher": {"id":"string","name":"string"},
  "capabilities": [{"id":"cap.resolve", "tags":["devtools"], "risk_tier":"medium"}],
  "platform_adapters": ["openai.tools","anthropic.skills","google.adk","mcp"],
  "licensing": {"model":"free|subscription|usage|enterprise"},
  "carp": {"min_version":"1.0"},
  "trace": {"min_version":"1.0"}
}
```

## Governance Metadata
- risk tier and capability tags
- approval requirements per action (`requires_approval` or high risk actions)
- adapter and action rate limits with window hints: `{ "limit": 10, "window_seconds": 300 }`
- required scopes per action to drive principal gating: `"required_scopes": ["repo.read"]`
- redaction rules for context blocks
- conformance fixtures describing adapter scope/approval/risk expectations (see `traces/fixtures/adapter_scopes.json`)

### Adapter stanza (example)
```json
{
  "openai.tools": {
    "rate_limit": {"limit": 50, "window_seconds": 3600},
    "actions": [
      {
        "name": "fetch_repo_status",
        "schema": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]},
        "constraints": [{"type": "scope", "value": ["repo.read"]}],
        "required_scopes": ["repo.read"],
        "rate_limit": {"limit": 10, "window_seconds": 300},
        "requires_approval": false
      }
    ]
  }
}
```

## Certification Hooks
- CARP and TRACE version minimums
- Tests covering adapters and policies
- Adapter scope/approval fixtures distributed alongside atlas for certification harnesses
- Hashes of artifacts to validate marketplace distribution
