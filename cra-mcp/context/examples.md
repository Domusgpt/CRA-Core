# Usage Examples

## Example 1: Successful Action

**User asks:** "Send a notification to the team"

**Good approach:**
```
1. Call cra_check:
   {
     "action": "notification.send",
     "parameters": {"channel": "team", "message": "Hello team!"}
   }

2. Response: {"allowed": true}

3. Proceed with action

4. Tell user: "I've sent the notification to the team channel."
```

## Example 2: Denied Action

**User asks:** "Delete all the old files"

**Good approach:**
```
1. Call cra_check:
   {
     "action": "files.delete_all",
     "parameters": {"filter": "older_than_30_days"}
   }

2. Response: {"allowed": false, "reason": "Bulk deletion requires approval"}

3. Tell user: "I can't delete all old files directly - this requires approval
   because it's a bulk deletion. Would you like me to:
   - List the files first so you can review them?
   - Request approval for this specific deletion?"
```

## Example 3: Rate Limited

**User asks:** "Send updates to all 500 subscribers"

**Good approach:**
```
1. Call cra_check for first batch

2. Response: {"allowed": true, "constraints": {"rate_limit": "10 per minute"}}

3. Tell user: "I can send these updates, but there's a rate limit of 10 per
   minute. For 500 subscribers, this will take about 50 minutes. Should I:
   - Start sending in batches?
   - Schedule this for overnight processing?
   - Do a smaller test batch first?"
```

## Example 4: Unknown Action

**User asks:** "Do something I've never done before"

**Good approach:**
```
1. Call cra_list_actions to see what's available

2. Call cra_help if still unsure:
   {"topic": "how do I handle new request types?"}

3. If no matching action exists, tell user:
   "I don't have a specific action for that in my current capabilities.
   Here's what I can do that might help: [list relevant actions]"
```

## Anti-Patterns (Don't Do This)

### ❌ Acting Without Checking
```
User: "Delete the file"
Bad: *deletes file immediately*
Good: *checks cra_check first, then deletes if allowed*
```

### ❌ Hiding Denials
```
User: "Send external email"
Bad: "Done!" (when actually blocked)
Good: "I can't send external emails directly - this requires approval.
      Would you like to proceed with the approval process?"
```

### ❌ Retry Loops
```
Bad:
  for i in range(100):
    try_action()  # hammering a rate limit

Good:
  result = cra_check(action)
  if result.rate_limited:
    tell_user_and_wait()
```
