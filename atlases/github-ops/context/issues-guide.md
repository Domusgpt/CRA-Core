# GitHub Issues Guide

## Overview

GitHub Issues is a flexible tracking system for bugs, features, and tasks. This guide covers best practices for issue management.

## Creating Issues

### Issue Titles
- Keep titles concise but descriptive (under 60 characters ideal)
- Use prefixes: `Bug:`, `Feature:`, `Docs:`, `Chore:`
- Include component name: `[Auth] Login fails on mobile`

### Issue Body Structure
```markdown
## Description
[Clear description of the issue]

## Steps to Reproduce (for bugs)
1. Step one
2. Step two
3. Step three

## Expected Behavior
[What should happen]

## Actual Behavior
[What actually happens]

## Environment
- OS:
- Browser:
- Version:
```

## Labels

### Standard Labels
- `bug` - Something isn't working
- `enhancement` - New feature request
- `documentation` - Documentation improvements
- `good first issue` - Good for newcomers
- `help wanted` - Extra attention needed
- `priority: high` - Urgent issues
- `priority: low` - Non-urgent issues

## Best Practices

1. **Search first** - Check for duplicates before creating
2. **One issue per problem** - Don't combine multiple issues
3. **Provide context** - Include screenshots, logs, examples
4. **Link related items** - Reference related issues/PRs
5. **Update status** - Keep issues current as work progresses

## Permissions

- All collaborators can create issues
- Maintainers can close issues
- Owners can lock/unlock issues
