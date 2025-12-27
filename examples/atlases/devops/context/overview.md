# DevOps Atlas

This Atlas governs DevOps operations for AI agents.

## Purpose

Enable AI agents to manage infrastructure and deployments while:
- Enforcing strict approval workflows for production changes
- Maintaining complete audit trails
- Preventing dangerous operations without oversight
- Enabling rapid incident response

## Capabilities

### Deployments
- **deploy.staging** - Deploy to staging environment
- **deploy.production** - Deploy to production (requires approval)
- **deploy.rollback** - Rollback to previous version

### Infrastructure
- **infra.scale** - Scale services up/down
- **infra.status** - Check infrastructure status

### Incident Management
- **incident.create** - Create incident tickets
- **incident.update** - Update incident status
- **incident.resolve** - Resolve incidents

### Observability
- **logs.search** - Search application logs
- **metrics.query** - Query metrics

## Risk Tiers

| Action | Risk Tier | Approval Required |
|--------|-----------|-------------------|
| deploy.staging | Medium | No |
| deploy.production | Critical | Yes |
| deploy.rollback | High | During incidents: No |
| infra.scale | High | If scaling > 2x: Yes |

## Constraints

1. Production deployments require human approval
2. All deployments are logged with full diff
3. Rollbacks automatically create incidents
4. Scaling limits enforced per service
