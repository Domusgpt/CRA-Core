# Deployment Guide

Complete guide to deploying CRA in development, staging, and production environments.

---

## Table of Contents

- [Quick Start](#quick-start)
- [Environment Configuration](#environment-configuration)
- [Development Setup](#development-setup)
- [Production Deployment](#production-deployment)
- [Docker Deployment](#docker-deployment)
- [Kubernetes Deployment](#kubernetes-deployment)
- [Database Setup](#database-setup)
- [Authentication Setup](#authentication-setup)
- [Observability Setup](#observability-setup)
- [Security Hardening](#security-hardening)
- [Scaling](#scaling)
- [Troubleshooting](#troubleshooting)

---

## Quick Start

### Minimal Development Setup

```bash
# Install CRA
pip install cra-core

# Start the runtime (in-memory storage, no auth)
cra runtime start --dev

# Verify it's running
curl http://localhost:8420/v1/health
```

### Minimal Production Setup

```bash
# Install with production dependencies
pip install cra-core[postgres,observability]

# Set required environment variables
export CRA_ENV=production
export CRA_AUTH__JWT_SECRET="your-secure-secret-here"
export CRA_STORAGE__POSTGRES_URL="postgresql://user:pass@host:5432/cra"

# Start the runtime
cra runtime start --host 0.0.0.0 --workers 4
```

---

## Environment Configuration

CRA uses environment variables with the `CRA_` prefix. Nested settings use `__` delimiter.

### Core Settings

| Variable | Default | Description |
|----------|---------|-------------|
| `CRA_ENV` | `development` | Environment: development, staging, production |
| `CRA_DEBUG` | `false` | Enable debug mode |
| `CRA_VERSION` | `0.1.0` | Version string |

### Runtime Settings

| Variable | Default | Description |
|----------|---------|-------------|
| `CRA_RUNTIME__HOST` | `127.0.0.1` | Server bind address |
| `CRA_RUNTIME__PORT` | `8420` | Server port |
| `CRA_RUNTIME__WORKERS` | `1` | Number of worker processes |
| `CRA_RUNTIME__RELOAD` | `false` | Auto-reload on code changes |
| `CRA_RUNTIME__LOG_LEVEL` | `info` | Log level |
| `CRA_RUNTIME__CORS_ORIGINS` | `["*"]` | CORS allowed origins |
| `CRA_RUNTIME__REQUEST_TIMEOUT_SECONDS` | `30` | Request timeout |
| `CRA_RUNTIME__MAX_REQUEST_SIZE_MB` | `10` | Max request size |

### Authentication Settings

| Variable | Default | Description |
|----------|---------|-------------|
| `CRA_AUTH__ENABLED` | `true` | Enable authentication |
| `CRA_AUTH__JWT_SECRET` | `change-me...` | JWT signing secret |
| `CRA_AUTH__JWT_ALGORITHM` | `HS256` | JWT algorithm |
| `CRA_AUTH__JWT_ACCESS_TOKEN_EXPIRE_MINUTES` | `60` | Access token TTL |
| `CRA_AUTH__JWT_REFRESH_TOKEN_EXPIRE_DAYS` | `7` | Refresh token TTL |
| `CRA_AUTH__API_KEY_ENABLED` | `true` | Enable API key auth |
| `CRA_AUTH__REQUIRE_AUTH_FOR_HEALTH` | `false` | Require auth for health endpoint |

### Storage Settings

| Variable | Default | Description |
|----------|---------|-------------|
| `CRA_STORAGE__BACKEND` | `memory` | Storage backend: memory, postgres |
| `CRA_STORAGE__POSTGRES_URL` | `null` | PostgreSQL connection URL |
| `CRA_STORAGE__POSTGRES_POOL_SIZE` | `10` | Connection pool size |
| `CRA_STORAGE__TRACE_RETENTION_DAYS` | `30` | Trace retention period |
| `CRA_STORAGE__SESSION_CLEANUP_INTERVAL_SECONDS` | `300` | Session cleanup interval |

### Observability Settings

| Variable | Default | Description |
|----------|---------|-------------|
| `CRA_OBSERVABILITY__OTEL_ENABLED` | `false` | Enable OpenTelemetry |
| `CRA_OBSERVABILITY__OTEL_ENDPOINT` | `localhost:4317` | OTEL collector endpoint |
| `CRA_OBSERVABILITY__OTEL_SERVICE_NAME` | `cra-runtime` | Service name |
| `CRA_OBSERVABILITY__SIEM_ENABLED` | `false` | Enable SIEM export |
| `CRA_OBSERVABILITY__SIEM_FORMAT` | `json` | SIEM format: json, cef, leef |
| `CRA_OBSERVABILITY__METRICS_ENABLED` | `true` | Enable metrics |
| `CRA_OBSERVABILITY__METRICS_PORT` | `9090` | Metrics endpoint port |

### Atlas Settings

| Variable | Default | Description |
|----------|---------|-------------|
| `CRA_ATLAS__AUTO_LOAD_PATHS` | `[]` | Paths to auto-load Atlases from |
| `CRA_ATLAS__CACHE_ENABLED` | `true` | Enable Atlas caching |
| `CRA_ATLAS__CACHE_TTL_SECONDS` | `3600` | Cache TTL |
| `CRA_ATLAS__VALIDATION_STRICT` | `true` | Strict manifest validation |

---

## Development Setup

### Using .env File

Create a `.env` file in your project root:

```bash
# .env
CRA_ENV=development
CRA_DEBUG=true
CRA_RUNTIME__RELOAD=true
CRA_RUNTIME__LOG_LEVEL=debug
CRA_AUTH__ENABLED=false
CRA_STORAGE__BACKEND=memory
```

### Start Development Server

```bash
# With auto-reload
cra runtime start --dev

# Or with explicit options
cra runtime start --reload --log-level debug
```

### Loading Test Atlases

```bash
# Load example Atlases
cra atlas load examples/atlases/customer-support
cra atlas load examples/atlases/devops

# Verify
cra atlas list
```

---

## Production Deployment

### Pre-Deployment Checklist

- [ ] PostgreSQL database provisioned
- [ ] Strong JWT secret generated
- [ ] CORS origins configured
- [ ] TLS/SSL termination configured
- [ ] Monitoring endpoints configured
- [ ] Log aggregation configured
- [ ] Backup strategy defined
- [ ] Rate limiting configured

### Generate Secure JWT Secret

```bash
# Generate a secure secret
python -c "import secrets; print(secrets.token_urlsafe(64))"
```

### Production Environment Variables

```bash
# Required
export CRA_ENV=production
export CRA_AUTH__JWT_SECRET="your-64-char-secure-secret"
export CRA_STORAGE__POSTGRES_URL="postgresql://user:pass@db.example.com:5432/cra"

# Recommended
export CRA_RUNTIME__HOST=0.0.0.0
export CRA_RUNTIME__WORKERS=4
export CRA_RUNTIME__CORS_ORIGINS='["https://app.example.com"]'
export CRA_RUNTIME__LOG_LEVEL=warning
export CRA_OBSERVABILITY__OTEL_ENABLED=true
export CRA_OBSERVABILITY__OTEL_ENDPOINT=otel-collector.example.com:4317
export CRA_OBSERVABILITY__SIEM_ENABLED=true
```

### Start Production Server

```bash
cra runtime start --host 0.0.0.0 --workers 4
```

---

## Docker Deployment

### Dockerfile

```dockerfile
FROM python:3.11-slim

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Python dependencies
COPY pyproject.toml .
RUN pip install --no-cache-dir .[postgres,observability]

# Copy application code
COPY . .

# Create non-root user
RUN useradd -m cra && chown -R cra:cra /app
USER cra

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s \
    CMD curl -f http://localhost:8420/v1/health || exit 1

EXPOSE 8420

CMD ["cra", "runtime", "start", "--host", "0.0.0.0"]
```

### Docker Compose

```yaml
# docker-compose.yml
version: '3.8'

services:
  cra:
    build: .
    ports:
      - "8420:8420"
    environment:
      CRA_ENV: production
      CRA_AUTH__JWT_SECRET: ${JWT_SECRET}
      CRA_STORAGE__BACKEND: postgres
      CRA_STORAGE__POSTGRES_URL: postgresql://cra:cra@db:5432/cra
      CRA_OBSERVABILITY__OTEL_ENABLED: "true"
      CRA_OBSERVABILITY__OTEL_ENDPOINT: otel-collector:4317
    depends_on:
      db:
        condition: service_healthy
    restart: unless-stopped
    deploy:
      replicas: 2
      resources:
        limits:
          cpus: '1'
          memory: 512M

  db:
    image: postgres:15
    environment:
      POSTGRES_USER: cra
      POSTGRES_PASSWORD: cra
      POSTGRES_DB: cra
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U cra"]
      interval: 5s
      timeout: 5s
      retries: 5

  otel-collector:
    image: otel/opentelemetry-collector:latest
    ports:
      - "4317:4317"
    volumes:
      - ./otel-config.yaml:/etc/otel/config.yaml
    command: ["--config", "/etc/otel/config.yaml"]

volumes:
  postgres_data:
```

### Build and Run

```bash
# Build
docker compose build

# Start services
docker compose up -d

# View logs
docker compose logs -f cra

# Scale workers
docker compose up -d --scale cra=4
```

---

## Kubernetes Deployment

### ConfigMap

```yaml
# configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: cra-config
data:
  CRA_ENV: "production"
  CRA_RUNTIME__HOST: "0.0.0.0"
  CRA_RUNTIME__WORKERS: "4"
  CRA_RUNTIME__LOG_LEVEL: "info"
  CRA_STORAGE__BACKEND: "postgres"
  CRA_OBSERVABILITY__OTEL_ENABLED: "true"
  CRA_OBSERVABILITY__METRICS_ENABLED: "true"
```

### Secret

```yaml
# secret.yaml
apiVersion: v1
kind: Secret
metadata:
  name: cra-secrets
type: Opaque
stringData:
  JWT_SECRET: "your-secure-jwt-secret"
  POSTGRES_URL: "postgresql://user:pass@postgres:5432/cra"
```

### Deployment

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: cra-runtime
  labels:
    app: cra
spec:
  replicas: 3
  selector:
    matchLabels:
      app: cra
  template:
    metadata:
      labels:
        app: cra
    spec:
      containers:
      - name: cra
        image: your-registry/cra:latest
        ports:
        - containerPort: 8420
          name: http
        - containerPort: 9090
          name: metrics
        envFrom:
        - configMapRef:
            name: cra-config
        env:
        - name: CRA_AUTH__JWT_SECRET
          valueFrom:
            secretKeyRef:
              name: cra-secrets
              key: JWT_SECRET
        - name: CRA_STORAGE__POSTGRES_URL
          valueFrom:
            secretKeyRef:
              name: cra-secrets
              key: POSTGRES_URL
        resources:
          requests:
            cpu: "100m"
            memory: "256Mi"
          limits:
            cpu: "1000m"
            memory: "512Mi"
        livenessProbe:
          httpGet:
            path: /v1/health
            port: 8420
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /v1/health
            port: 8420
          initialDelaySeconds: 5
          periodSeconds: 10
```

### Service

```yaml
# service.yaml
apiVersion: v1
kind: Service
metadata:
  name: cra-runtime
spec:
  selector:
    app: cra
  ports:
  - name: http
    port: 8420
    targetPort: 8420
  - name: metrics
    port: 9090
    targetPort: 9090
```

### Ingress

```yaml
# ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: cra-ingress
  annotations:
    kubernetes.io/ingress.class: nginx
    cert-manager.io/cluster-issuer: letsencrypt-prod
spec:
  tls:
  - hosts:
    - cra.example.com
    secretName: cra-tls
  rules:
  - host: cra.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: cra-runtime
            port:
              number: 8420
```

### Deploy to Kubernetes

```bash
# Apply configuration
kubectl apply -f configmap.yaml
kubectl apply -f secret.yaml
kubectl apply -f deployment.yaml
kubectl apply -f service.yaml
kubectl apply -f ingress.yaml

# Check status
kubectl get pods -l app=cra
kubectl logs -l app=cra -f
```

---

## Database Setup

### PostgreSQL Schema

CRA automatically creates required tables on startup. For manual setup:

```sql
-- Sessions table
CREATE TABLE IF NOT EXISTS cra_sessions (
    session_id UUID PRIMARY KEY,
    agent_id VARCHAR(255) NOT NULL,
    atlas_id VARCHAR(255),
    goal TEXT,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Trace events table
CREATE TABLE IF NOT EXISTS cra_trace_events (
    event_id UUID PRIMARY KEY,
    session_id UUID REFERENCES cra_sessions(session_id),
    trace_id UUID NOT NULL,
    span_id UUID NOT NULL,
    parent_span_id UUID,
    event_type VARCHAR(100) NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_trace_events_session ON cra_trace_events(session_id);
CREATE INDEX idx_trace_events_trace ON cra_trace_events(trace_id);
CREATE INDEX idx_trace_events_type ON cra_trace_events(event_type);
CREATE INDEX idx_trace_events_timestamp ON cra_trace_events(timestamp);
CREATE INDEX idx_sessions_status ON cra_sessions(status);
CREATE INDEX idx_sessions_agent ON cra_sessions(agent_id);

-- API keys table (if using API key auth)
CREATE TABLE IF NOT EXISTS cra_api_keys (
    key_id UUID PRIMARY KEY,
    key_hash VARCHAR(64) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    scopes TEXT[] NOT NULL DEFAULT '{}',
    roles TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE,
    last_used_at TIMESTAMP WITH TIME ZONE,
    revoked BOOLEAN DEFAULT FALSE
);
```

### Database Migrations

```bash
# Run migrations (future feature)
cra db migrate

# Check migration status
cra db status
```

### Connection Pooling

For high-traffic deployments, use PgBouncer:

```ini
# pgbouncer.ini
[databases]
cra = host=postgres port=5432 dbname=cra

[pgbouncer]
listen_addr = 0.0.0.0
listen_port = 6432
auth_type = md5
pool_mode = transaction
max_client_conn = 1000
default_pool_size = 20
```

---

## Authentication Setup

### JWT Authentication

1. Generate a secure secret:
   ```bash
   export CRA_AUTH__JWT_SECRET=$(python -c "import secrets; print(secrets.token_urlsafe(64))")
   ```

2. Obtain tokens via API:
   ```bash
   curl -X POST http://localhost:8420/v1/auth/login \
     -H "Content-Type: application/json" \
     -d '{"username": "admin", "password": "password"}'
   ```

3. Use token in requests:
   ```bash
   curl http://localhost:8420/v1/sessions \
     -H "Authorization: Bearer eyJ..."
   ```

### API Key Authentication

1. Generate an API key:
   ```bash
   cra auth create-key --name "production-agent" --roles agent --scopes "atlas:read,session:write"
   ```

2. Use API key in requests:
   ```bash
   curl http://localhost:8420/v1/sessions \
     -H "X-API-Key: cra_..."
   ```

### Role-Based Access Control

Built-in roles:
- `admin` — Full access
- `developer` — Read/write access except admin functions
- `agent` — Limited to session and atlas operations
- `viewer` — Read-only access
- `auditor` — Read access to traces

---

## Observability Setup

### OpenTelemetry

1. Deploy OTEL Collector:
   ```yaml
   # otel-config.yaml
   receivers:
     otlp:
       protocols:
         grpc:
           endpoint: 0.0.0.0:4317

   processors:
     batch:
       timeout: 1s

   exporters:
     jaeger:
       endpoint: jaeger:14250
       tls:
         insecure: true

   service:
     pipelines:
       traces:
         receivers: [otlp]
         processors: [batch]
         exporters: [jaeger]
   ```

2. Configure CRA:
   ```bash
   export CRA_OBSERVABILITY__OTEL_ENABLED=true
   export CRA_OBSERVABILITY__OTEL_ENDPOINT=otel-collector:4317
   ```

### Prometheus Metrics

Metrics are exposed at `/metrics` on the metrics port (default: 9090).

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'cra'
    static_configs:
      - targets: ['cra-runtime:9090']
```

Key metrics:
- `cra_sessions_total` — Total sessions created
- `cra_trace_events_total` — Total trace events
- `cra_resolve_duration_seconds` — CARP resolution latency
- `cra_execute_duration_seconds` — Action execution latency
- `cra_policy_violations_total` — Policy violations

### SIEM Integration

Configure SIEM export:

```bash
export CRA_OBSERVABILITY__SIEM_ENABLED=true
export CRA_OBSERVABILITY__SIEM_FORMAT=json  # or cef, leef
```

Events are logged to stdout in the configured format for ingestion by log aggregators.

---

## Security Hardening

### Production Checklist

1. **Authentication**
   - [ ] Strong JWT secret (64+ chars)
   - [ ] Short token expiration
   - [ ] API keys rotated regularly

2. **Network**
   - [ ] TLS termination at load balancer
   - [ ] Internal services on private network
   - [ ] Firewall rules limiting access

3. **CORS**
   - [ ] Specific origins, not `*`
   - [ ] Only required methods allowed

4. **Database**
   - [ ] Encrypted connections (SSL)
   - [ ] Least-privilege database user
   - [ ] Regular backups

5. **Secrets**
   - [ ] Secrets in vault/secret manager
   - [ ] No secrets in code or logs
   - [ ] Rotation policy

### Rate Limiting

Configure at reverse proxy level:

```nginx
# nginx.conf
limit_req_zone $binary_remote_addr zone=cra:10m rate=10r/s;

server {
    location /v1/ {
        limit_req zone=cra burst=20 nodelay;
        proxy_pass http://cra-runtime:8420;
    }
}
```

---

## Scaling

### Horizontal Scaling

CRA is stateless and scales horizontally:

```bash
# Docker Compose
docker compose up -d --scale cra=4

# Kubernetes
kubectl scale deployment cra-runtime --replicas=10
```

### Vertical Scaling

Adjust worker count based on CPU cores:

```bash
# Rule of thumb: 2 * CPU cores + 1
CRA_RUNTIME__WORKERS=9  # For 4-core machine
```

### Database Scaling

For high-throughput:
1. Use read replicas for trace queries
2. Partition trace_events by timestamp
3. Implement trace archival to cold storage

---

## Troubleshooting

### Common Issues

#### Connection Refused

```bash
# Check if server is running
curl http://localhost:8420/v1/health

# Check logs
docker compose logs cra
```

#### Database Connection Failed

```bash
# Test PostgreSQL connection
psql $CRA_STORAGE__POSTGRES_URL -c "SELECT 1"

# Check connection string format
# postgresql://user:password@host:port/database
```

#### JWT Token Expired

```bash
# Refresh token
curl -X POST http://localhost:8420/v1/auth/refresh \
  -H "Authorization: Bearer <refresh_token>"
```

#### Atlas Not Found

```bash
# List loaded Atlases
cra atlas list

# Load Atlas
cra atlas load /path/to/atlas

# Validate Atlas
cra atlas validate /path/to/atlas
```

### Debug Mode

Enable debug logging:

```bash
export CRA_DEBUG=true
export CRA_RUNTIME__LOG_LEVEL=debug
cra runtime start
```

### Health Checks

```bash
# Basic health
curl http://localhost:8420/v1/health

# Detailed health (with database check)
curl http://localhost:8420/v1/health?detailed=true
```

---

*For more information, see the [API Reference](API.md) and [CLI Reference](CLI.md).*
