# CRA Runtime API v0.1

The runtime exposes a small HTTP-like interface and a Python API used by the CLI. All transport carriers must emit TRACE events; LLM narration is never authoritative.

## Key Operations
- **POST /carp/resolve** — accept CARP resolve requests, return CARP response with permitted context/actions
- **POST /carp/execute** — optional execution hook for follow-on actions
- **GET /trace/stream** — server-sent events / chunked JSONL of TRACE telemetry
- **GET /trace/replay** — replay a prior trace_id with artifacts

## Runtime Rules
- All tool calls go through the runtime and are authorized by the most recent CARP resolution
- TRACE events are immutable and append-only
- Every request is linked to a session id, trace id, and span hierarchy
- Policies and approvals are evaluated before granting actions

## In-Memory Reference Implementation
This repository includes an in-memory runtime with:
- CARP request/response models
- TRACE emitter writing JSONL to `traces/<trace_id>.jsonl`
- Simple policy checks based on atlas capability risk tier
