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
- approval requirements and rate limits per adapter
- redaction rules for context blocks

## Certification Hooks
- CARP and TRACE version minimums
- Tests covering adapters and policies
- Hashes of artifacts to validate marketplace distribution
