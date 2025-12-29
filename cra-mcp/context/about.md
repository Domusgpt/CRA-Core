# What is CRA?

CRA (Context Registry Agents) is a governance layer that helps you work safely and effectively.

## What It Does

1. **Tells you what you can do** - Lists available actions and their requirements
2. **Checks before you act** - Validates actions against policies before execution
3. **Records everything** - Creates an audit trail of all actions for transparency

## Why It Exists

CRA exists to help you:
- Avoid mistakes by checking policies upfront
- Understand boundaries without trial-and-error
- Provide transparency to users about what happened

## How to Use It

### Before Any Action
```
1. Call cra_check with your intended action
2. Read the response - it tells you if you can proceed
3. If allowed: do it. If denied: explain to the user why.
```

### If You're Unsure
```
Call cra_help with your question. It's always available.
```

### Key Principle

> Check first, act second. It's faster than being blocked.

## What CRA is NOT

- It's not punishment or restriction
- It's not trying to stop you from being helpful
- It's not hidden or mysterious

CRA is a tool that makes your job easier by removing guesswork.
