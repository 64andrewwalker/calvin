---
description: Containerization
---

# [2-INIT] Containerization Setup

> **Stage**: Initialize | **Category**: Infrastructure | **Frequency**: Deployment setup

Create production-ready container configuration. Track in `TODO.md`.

## Dockerfile Best Practices

1. **Multi-stage Build**:
   - Separate build and runtime stages
   - Minimize final image size
   - Cache dependency layers effectively
2. **Security Hardening**:
   - Non-root user
   - Read-only filesystem where possible
   - No secrets in image layers
   - Minimal base image (alpine, distroless)
3. **Optimization**:
   - .dockerignore configuration
   - Layer ordering for cache efficiency
   - Health check implementation

## Docker Compose (Development)

```yaml
services:
  app:
    build: .
    volumes: [./src:/app/src]  # Hot reload
    environment: [NODE_ENV=development]
  db:
    image: postgres:15-alpine
    volumes: [pgdata:/var/lib/postgresql/data]
  redis:
    image: redis:7-alpine
```

## Production Configuration

1. **Orchestration Ready**:
   - Kubernetes manifests or Helm charts
   - Resource limits and requests
   - Liveness/readiness probes
   - Graceful shutdown handling
2. **Secrets Management**:
   - External secret injection
   - No hardcoded credentials

## Output

- `Dockerfile` (multi-stage, optimized)
- `docker-compose.yml` (development)
- `docker-compose.prod.yml` (production simulation)
- `.dockerignore`
- `k8s/` or `helm/` (if applicable)
- `docs/container-guide.md`
