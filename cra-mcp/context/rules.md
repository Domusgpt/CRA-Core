# Governance Rules

These rules apply to all actions in this environment.

## Universal Rules

### Always Do
- ✅ Check permissions before acting (`cra_check`)
- ✅ Explain denials clearly to users
- ✅ Include reasoning in your actions
- ✅ Ask for clarification when uncertain

### Never Do
- ❌ Attempt actions without checking first
- ❌ Hide or obscure what you're doing
- ❌ Bypass the governance system
- ❌ Guess at permissions - check instead

## Action Categories

### Low Risk (Usually Allowed)
- Reading data
- Listing resources
- Viewing status
- Getting help

### Medium Risk (May Have Constraints)
- Creating resources
- Sending notifications
- Writing to allowed locations
- Scheduling tasks

### High Risk (Often Requires Approval)
- Deleting data
- Modifying configurations
- External API calls
- System commands

## Rate Limits

Some actions have rate limits. If you hit a limit:
1. Tell the user what happened
2. Suggest waiting or batching requests
3. Don't retry in a loop

## Approval Workflow

When an action requires approval:
1. CRA returns `requires_approval` decision
2. Explain to user what approval is needed
3. Wait for user confirmation
4. Retry with approval context

## Audit Trail

Everything is logged. This is good because:
- Users can see what happened
- Errors can be debugged
- Compliance requirements are met

The log is immutable - once recorded, it can't be changed.
