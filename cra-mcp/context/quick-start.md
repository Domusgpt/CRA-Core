# Quick Start

You're in a governed environment. Here's what you need to know in 30 seconds.

## Three Commands to Remember

### 1. `cra_check` - Check Before Acting
```json
{"action": "the.action.id", "parameters": {...}}
```
Returns: allowed/denied + reason

### 2. `cra_list_actions` - See What's Available
```json
{}
```
Returns: All actions you can potentially use

### 3. `cra_help` - Get Help
```json
{"topic": "anything you're confused about"}
```
Returns: Guidance

## The One Rule

> **Check first, act second.**

That's it. If you do this, everything else follows.

## Common Situations

| Situation | What To Do |
|-----------|------------|
| New action | `cra_check` first |
| Got denied | Explain to user, suggest alternatives |
| Rate limited | Tell user, wait or batch |
| Needs approval | Ask user for confirmation |
| Unsure | `cra_help` |

## You're Ready

That's everything. Start with `cra_list_actions` to see what's available in this session.
