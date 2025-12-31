# Dogfooding Task

Give this task to a fresh LLM session on this repo.

---

## Task: Add Context Staleness Event

Add a new TRACE event type called `context.stale` that should be emitted when the system detects that injected context may be outdated.

The event payload should include:
- `context_id`: which context block is stale
- `reason`: why it's considered stale (e.g., "source_file_changed", "ttl_expired")
- `source_file`: the file that changed (if applicable)
- `last_verified`: when the context was last verified

Requirements:
1. Add the new event type to the appropriate location
2. Ensure it integrates with the existing event system
3. Add at least one test for the new event type
4. Make sure all existing tests still pass

---

## How to Run the Experiment

### Control Group (No Context)
1. Open new Claude Code session on this repo
2. Clear any existing CLAUDE.md or rename it temporarily
3. Give the task above
4. Record: files touched, approach taken, errors made, tests written, final result

### Experiment Group (With Context)
1. Open new Claude Code session on this repo
2. Copy `experiments/dogfood_context.md` content into CLAUDE.md
3. Give the same task
4. Record: files touched, approach taken, errors made, tests written, final result

### What to Observe

| Observation | Control | Experiment |
|-------------|---------|------------|
| First file opened | ? | ? |
| Found EventType enum | ? | ? |
| Followed existing pattern | ? | ? |
| Used compute_hash correctly | ? | ? |
| Added tests | ? | ? |
| Tests pass | ? | ? |
| Chain integrity maintained | ? | ? |
| Number of errors/retries | ? | ? |

### Success

If the Experiment group consistently:
- Finds the right file faster
- Follows patterns correctly
- Makes fewer mistakes
- Maintains chain integrity

Then context injection WORKS.
