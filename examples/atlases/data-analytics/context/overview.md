# Data Analytics Atlas

This Atlas governs data analytics operations for AI agents.

## Purpose

Enable AI agents to analyze data and generate insights while:
- Enforcing data access controls
- Preventing expensive runaway queries
- Protecting sensitive data through redaction
- Maintaining complete audit trails

## Capabilities

### Querying
- **query.run** - Execute SQL queries
- **query.explain** - Explain query execution plan

### Reporting
- **report.generate** - Generate reports
- **report.schedule** - Schedule recurring reports

### Dashboards
- **dashboard.create** - Create dashboards
- **dashboard.share** - Share with team members

### Pipelines
- **pipeline.status** - Check pipeline status
- **pipeline.trigger** - Trigger pipeline runs

### Exports
- **export.csv** - Export data as CSV
- **export.json** - Export data as JSON

## Data Access Levels

| Level | Description | Approval |
|-------|-------------|----------|
| Public | Aggregated, non-sensitive | None |
| Internal | Company data | None |
| Confidential | Customer data | Role-based |
| Restricted | PII, financial | Explicit approval |

## Query Constraints

1. Maximum query runtime: 5 minutes
2. Maximum result rows: 100,000
3. No SELECT * on large tables
4. JOINs limited to 5 tables
