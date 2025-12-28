# GitHub Pull Requests Guide

## Overview

Pull Requests (PRs) are proposals to merge code changes. They enable code review, discussion, and CI validation before merging.

## Creating Pull Requests

### PR Title Format
```
type(scope): description

Examples:
- feat(auth): add OAuth2 login support
- fix(api): handle null response in user endpoint
- docs(readme): update installation instructions
- refactor(db): simplify query builder
```

### PR Body Template
```markdown
## Summary
[1-3 sentences describing the change]

## Changes
- [List of specific changes]
- [Another change]

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing completed

## Screenshots (if applicable)
[Before/after screenshots]

## Related Issues
Closes #123
Related to #456
```

## Code Review Guidelines

### For Authors
1. Keep PRs small (< 400 lines ideal)
2. Write clear descriptions
3. Respond to feedback promptly
4. Mark conversations as resolved

### For Reviewers
1. Review within 24 hours
2. Be constructive and specific
3. Approve or request changes clearly
4. Use inline comments for specific lines

## Merge Strategies

### Merge Commit
- Creates a merge commit
- Preserves all individual commits
- Best for: feature branches with meaningful history

### Squash and Merge
- Combines all commits into one
- Clean main branch history
- Best for: most feature work

### Rebase and Merge
- Replays commits on top of base
- Linear history, no merge commits
- Best for: small, atomic changes

## Branch Protection

Protected branches can require:
- Pull request reviews (1-6 approvers)
- Status checks to pass
- Up-to-date branches
- Signed commits
- Linear history

## Permissions

- Anyone with write access can create PRs
- Reviewers must have read access minimum
- Merge requires appropriate permissions + reviews
