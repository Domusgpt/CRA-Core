# GitHub Repository Settings Guide

## Overview

Repository settings control access, security, and behavior of your GitHub repository. Changes here can have significant impact.

## General Settings

### Repository Name and Description
- Names should be lowercase with hyphens
- Descriptions should be concise and clear
- Add topics for discoverability

### Features
- **Wikis**: Enable for documentation
- **Issues**: Enable for bug tracking
- **Projects**: Enable for project management
- **Discussions**: Enable for community Q&A

## Branch Protection Rules

### Recommended Settings for `main`

```yaml
protection_rules:
  require_pull_request:
    required_approving_review_count: 1
    dismiss_stale_reviews: true
    require_code_owner_reviews: true
  require_status_checks:
    strict: true
    contexts:
      - "ci/build"
      - "ci/test"
  require_signed_commits: false
  require_linear_history: false
  allow_force_pushes: false
  allow_deletions: false
```

### High-Security Configuration

For production or sensitive repos:
- Require 2+ approving reviews
- Require code owner reviews
- Require signed commits
- Require linear history
- Include administrators in restrictions

## Access Levels

| Role | Read | Triage | Write | Maintain | Admin |
|------|------|--------|-------|----------|-------|
| View code | ✓ | ✓ | ✓ | ✓ | ✓ |
| Create issues | ✓ | ✓ | ✓ | ✓ | ✓ |
| Manage issues | | ✓ | ✓ | ✓ | ✓ |
| Push code | | | ✓ | ✓ | ✓ |
| Manage settings | | | | ✓ | ✓ |
| Delete repo | | | | | ✓ |

## Webhooks

Common webhook events:
- `push` - Code pushed
- `pull_request` - PR opened/closed/merged
- `issues` - Issue created/closed
- `release` - Release published

## Security

### Dependabot
- Enable security advisories
- Enable dependency updates
- Configure update schedule

### Secret Scanning
- Enable for all repositories
- Enable push protection

### Code Scanning
- Enable CodeQL analysis
- Run on push and PR

## DANGER ZONE

These actions are destructive:
- **Transfer ownership**: Moves repo to another owner
- **Archive repository**: Makes repo read-only
- **Delete repository**: Permanently deletes repo

Always require confirmation for these actions.
