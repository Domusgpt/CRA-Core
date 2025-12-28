# CRA CLI Reference

Complete command-line interface documentation for CRA.

---

## Installation

```bash
pip install cra-core
```

Verify installation:

```bash
cra --version
```

---

## Global Options

| Option | Description |
|--------|-------------|
| `--version`, `-v` | Show version and exit |
| `--help` | Show help message |

---

## Commands

### cra doctor

Check system health and configuration.

```bash
cra doctor [OPTIONS]
```

**Options**

| Option | Description |
|--------|-------------|
| `--json` | Output in JSON format |
| `--verbose` | Show detailed diagnostics |

**Example**

```bash
$ cra doctor

CRA Doctor - System Check
=========================
Runtime:     http://localhost:8420  [OK]
Version:     0.1.0                  [OK]
CARP:        1.0                    [OK]
TRACE:       1.0                    [OK]
Config:      ./cra.config.json      [FOUND]
Trace Dir:   ./cra.trace/           [WRITABLE]
Atlases:     2 loaded               [OK]

All checks passed.
```

---

### cra init

Initialize a new CRA project.

```bash
cra init [OPTIONS] [PATH]
```

**Arguments**

| Argument | Description |
|----------|-------------|
| `PATH` | Directory to initialize (default: current) |

**Options**

| Option | Description |
|--------|-------------|
| `--runtime-url URL` | Runtime URL (default: http://localhost:8420) |
| `--force`, `-f` | Overwrite existing files |

**Example**

```bash
$ cra init my-project

Initializing CRA project in ./my-project...
Created: cra.config.json     # Runtime configuration
Created: agents.md           # Agent behavior contract
Created: cra.trace/          # Local trace storage
Created: cra.atlases.lock    # Atlas version lock

Project initialized. Run 'cra doctor' to verify.
```

**Generated Files**

`cra.config.json`:
```json
{
  "cra_version": "0.1.0",
  "runtime": {
    "url": "http://localhost:8420",
    "timeout_ms": 30000
  },
  "trace": {
    "directory": "./cra.trace",
    "retention_days": 30,
    "streaming": true
  },
  "atlases": [],
  "policies": {
    "default_risk_tier": "medium",
    "require_approval_for": ["high", "critical"]
  }
}
```

---

### cra resolve

Resolve context and permissions for a goal.

```bash
cra resolve [OPTIONS]
```

**Options**

| Option | Description |
|--------|-------------|
| `--goal`, `-g` TEXT | Goal description (required) |
| `--atlas`, `-a` TEXT | Atlas ID to use |
| `--capability`, `-c` TEXT | Filter by capability |
| `--risk-tier`, `-r` TEXT | Risk tier: low, medium, high, critical |
| `--json` | Output raw JSON |
| `--stream` | Stream TRACE events |

**Example**

```bash
$ cra resolve --goal "Deploy service to staging" --atlas com.example.devops

Resolution ID: 789e0123-e89b-12d3-a456-426614174000
Confidence:    95%

Context Blocks:
  - deployment-rules: Staging deployment guidelines
  - service-config: Service configuration reference

Allowed Actions:
  ✓ deploy.staging (requires_approval: false)
  ✓ deploy.status (requires_approval: false)

Denied Patterns:
  ✗ deploy.production: Production access not in scope
  ✗ db.*: Database operations blocked

Trace ID: 6ba7b810-9dad-11d1-80b4-00c04fd430c8
```

**JSON Output**

```bash
$ cra resolve --goal "Deploy to staging" --json

{
  "resolution_id": "789e0123-e89b-12d3-a456-426614174000",
  "confidence": 0.95,
  "context_blocks": [...],
  "allowed_actions": [...],
  "denylist": [...],
  "trace_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
}
```

---

### cra execute

Execute a granted action.

```bash
cra execute [OPTIONS] ACTION_ID
```

**Arguments**

| Argument | Description |
|----------|-------------|
| `ACTION_ID` | Action to execute |

**Options**

| Option | Description |
|--------|-------------|
| `--resolution-id`, `-r` UUID | Resolution ID (required) |
| `--param`, `-p` KEY=VALUE | Action parameters (repeatable) |
| `--json` | Output raw JSON |
| `--stream` | Stream TRACE events |

**Example**

```bash
$ cra execute deploy.staging \
    --resolution-id 789e0123-e89b-12d3-a456-426614174000 \
    --param service=api \
    --param version=1.2.3

Execution ID: def45678-e89b-12d3-a456-426614174000
Status:       completed

Result:
  deployment_id: DEP-456
  environment: staging
  status: running

Trace ID: 6ba7b810-9dad-11d1-80b4-00c04fd430c8
```

---

### cra trace

Manage and view traces.

#### cra trace tail

Stream trace events.

```bash
cra trace tail [OPTIONS]
```

**Options**

| Option | Description |
|--------|-------------|
| `--trace-id`, `-t` UUID | Trace ID to stream |
| `--follow`, `-f` | Follow new events (like tail -f) |
| `--event-type`, `-e` TEXT | Filter by event type |
| `--severity`, `-s` TEXT | Filter by severity |
| `--json` | Output raw JSONL |

**Example**

```bash
$ cra trace tail --trace-id 6ba7b810-9dad-11d1 --follow

[10:00:01] trace.session.started    info   Session created
[10:00:02] trace.carp.resolve       info   Resolution requested
[10:00:02] trace.carp.resolve       info   Resolution returned (confidence: 95%)
[10:00:03] trace.action.invoked     info   Action: deploy.staging
[10:00:05] trace.action.completed   info   Deployment successful
```

#### cra trace list

List recent traces.

```bash
cra trace list [OPTIONS]
```

**Options**

| Option | Description |
|--------|-------------|
| `--session-id` UUID | Filter by session |
| `--limit`, `-n` INT | Max traces to show (default: 20) |
| `--json` | Output raw JSON |

**Example**

```bash
$ cra trace list --limit 5

Trace ID                              Session                               Events  First Event
------------------------------------  ------------------------------------  ------  -----------
6ba7b810-9dad-11d1-80b4-00c04fd430c8  550e8400-e29b-41d4-a716-446655440000     42  10 min ago
abc12345-1234-5678-90ab-cdef12345678  661e9511-f3ac-52e5-b827-557766551111     15  1 hour ago
```

#### cra trace replay

Replay a trace for testing.

```bash
cra trace replay [OPTIONS] TRACE_ID
```

**Options**

| Option | Description |
|--------|-------------|
| `--manifest` PATH | Golden trace manifest file |
| `--compare` | Compare with expected output |
| `--output`, `-o` PATH | Write replay output to file |

**Example**

```bash
$ cra trace replay 6ba7b810-9dad-11d1 --compare

Replaying trace 6ba7b810-9dad-11d1...

Events replayed: 42
Differences found: 0

✓ Replay matches expected output
```

---

### cra atlas

Manage Atlases.

#### cra atlas list

List registered Atlases.

```bash
cra atlas list [OPTIONS]
```

**Example**

```bash
$ cra atlas list

Registered Atlases (3)
ID                            Version  Name                  Capabilities        Adapters
----------------------------  -------  --------------------  ------------------  --------
com.example.customer-support  1.0.0    Customer Support      ticket.*, kb.*      openai, anthropic
com.example.devops            1.0.0    DevOps                deploy.*, infra.*   openai, mcp
com.example.data-analytics    1.0.0    Data Analytics        query.*, report.*   openai, anthropic
```

#### cra atlas load

Load an Atlas from a path.

```bash
cra atlas load PATH
```

**Example**

```bash
$ cra atlas load ./examples/atlases/customer-support

Atlas loaded successfully!

  ID:           com.example.customer-support
  Version:      1.0.0
  Name:         Customer Support Atlas
  Capabilities: ticket.create, ticket.update, ticket.resolve, kb.search
  Adapters:     openai, anthropic, mcp
```

#### cra atlas info

Show Atlas details.

```bash
cra atlas info ATLAS_ID
```

**Example**

```bash
$ cra atlas info com.example.customer-support

Customer Support Atlas
ID: com.example.customer-support v1.0.0

Tools for handling customer support operations including
ticket management and knowledge base access.

Capabilities:
  - ticket.create
  - ticket.update
  - ticket.resolve
  - kb.search
  - kb.retrieve

Resources:
  Context Packs: 3
  Policies:      2

Adapters:
  - openai
  - anthropic
  - mcp

Certification:
  CARP Compliant:  Yes
  TRACE Compliant: Yes
```

#### cra atlas unload

Unload an Atlas.

```bash
cra atlas unload ATLAS_ID
```

#### cra atlas emit

Emit Atlas in platform-specific format.

```bash
cra atlas emit ATLAS_ID [OPTIONS]
```

**Options**

| Option | Description |
|--------|-------------|
| `--platform`, `-p` TEXT | Platform: openai, anthropic, google_adk, mcp |
| `--output`, `-o` PATH | Write to file |

**Example**

```bash
$ cra atlas emit com.example.customer-support -p openai -o tools.json

Output written to: tools.json
```

#### cra atlas context

Show context blocks from an Atlas.

```bash
cra atlas context ATLAS_ID [OPTIONS]
```

**Options**

| Option | Description |
|--------|-------------|
| `--capability`, `-c` TEXT | Filter by capability |

#### cra atlas actions

Show allowed actions from an Atlas.

```bash
cra atlas actions ATLAS_ID [OPTIONS]
```

---

### cra template

Generate agent templates.

#### cra template list

List supported frameworks.

```bash
$ cra template list

Supported Frameworks
Framework    Description                          Version
-----------  -----------------------------------  -------
openai_gpt   OpenAI GPT Actions (Custom GPTs)    2024-01
langchain    LangChain/LangGraph agents          0.1.0
crewai       CrewAI multi-agent crews            0.28.0
```

#### cra template generate

Generate template from an Atlas.

```bash
cra template generate ATLAS_PATH [OPTIONS]
```

**Options**

| Option | Description |
|--------|-------------|
| `--framework`, `-f` TEXT | Target framework |
| `--output`, `-o` PATH | Output directory |
| `--langgraph/--no-langgraph` | Use LangGraph (for langchain) |

**Example**

```bash
$ cra template generate ./examples/atlases/customer-support -f langchain

Generating langchain template for Customer Support Atlas...

Generated 4 files

Generated Files:
  📄 generated/langchain/cra_tools.py
     LangChain tools backed by CRA
  📄 generated/langchain/cra_agent.py
     LangGraph agent with CRA governance
  🔧 generated/langchain/main.py
     Example usage script
  📄 generated/langchain/requirements.txt
     Python dependencies

Dependencies:
  - cra>=0.1.0
  - httpx>=0.25.0
  - langchain>=0.1.0
  - langchain-openai>=0.0.5
  - langgraph>=0.0.20

Next Steps:
  cd generated/langchain
  pip install -r requirements.txt
  python main.py
```

#### cra template info

Show framework template details.

```bash
cra template info FRAMEWORK
```

---

### cra replay

Replay traces for testing.

```bash
cra replay [OPTIONS]
```

**Options**

| Option | Description |
|--------|-------------|
| `--manifest`, `-m` PATH | Golden trace manifest |
| `--trace-id`, `-t` UUID | Trace ID to replay |
| `--compare`, `-c` | Compare with expected |
| `--output`, `-o` PATH | Output file |

---

## Configuration File

CRA reads configuration from `cra.config.json`:

```json
{
  "cra_version": "0.1.0",
  "runtime": {
    "url": "http://localhost:8420",
    "timeout_ms": 30000
  },
  "trace": {
    "directory": "./cra.trace",
    "retention_days": 30,
    "streaming": true
  },
  "atlases": [
    {
      "path": "./atlases/customer-support",
      "auto_load": true
    }
  ],
  "policies": {
    "default_risk_tier": "medium",
    "require_approval_for": ["high", "critical"]
  }
}
```

---

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `CRA_RUNTIME_URL` | Runtime server URL | http://localhost:8420 |
| `CRA_CONFIG_PATH` | Config file path | ./cra.config.json |
| `CRA_TRACE_DIR` | Trace storage directory | ./cra.trace |
| `CRA_API_KEY` | API key for authentication | — |
| `CRA_JWT_TOKEN` | JWT token for authentication | — |

---

## Exit Codes

| Code | Description |
|------|-------------|
| 0 | Success |
| 1 | General error |
| 2 | Configuration error |
| 3 | Connection error |
| 4 | Authentication error |
| 5 | Resolution denied |

---

## Shell Completion

Enable shell completion:

**Bash**
```bash
eval "$(_CRA_COMPLETE=bash_source cra)"
```

**Zsh**
```bash
eval "$(_CRA_COMPLETE=zsh_source cra)"
```

**Fish**
```bash
_CRA_COMPLETE=fish_source cra | source
```

---

*For more examples, see the [Quick Start](../README.md#quick-start).*
