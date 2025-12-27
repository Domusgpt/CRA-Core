# Hello World Skill

## Overview

This skill allows you to send greetings.

## Available Actions

### greeting_send

Send a greeting message.

**Parameters:**
- `message` (required): The greeting message (max 1000 chars)
- `recipient` (optional): Name of the recipient

**Example:**
```
Use greeting_send with message "Hello, World!"
```

## Constraints

- Messages must be appropriate and professional
- Maximum message length: 1000 characters

## Deny Patterns

Do NOT attempt:
- Sending empty messages
- Including inappropriate content
- Bypassing content filters
