---
description: Environment Configuration
---

# [2-INIT] Environment & Configuration Management

> **Stage**: Initialize | **Category**: Configuration | **Frequency**: Project setup

Standardize configuration handling. Track in `TODO.md`.

## Configuration Audit

1. **Source Identification**:
   - Environment variables
   - Config files (JSON, YAML, TOML)
   - Command-line arguments
   - Remote config services
2. **Security Review**:
   - Secrets in code/config files
   - Exposed sensitive defaults
   - Missing production overrides

## Configuration Architecture

1. **Hierarchy**:

   ```text
   defaults → environment → local overrides → runtime
   ```

2. **Validation**:
   - Schema validation on startup
   - Required vs optional
   - Type coercion rules
3. **Secret Management**:
   - Environment injection
   - Secret manager integration (Vault, AWS SM, etc.)
   - Local development secrets (.env.local, gitignored)

## Implementation

```text
config/
├── default.ts      # Base configuration
├── development.ts  # Dev overrides
├── production.ts   # Prod overrides
├── schema.ts       # Validation schema
└── index.ts        # Loader with validation
```

## Output

- `config/` - Restructured configuration
- `.env.example` - Documented template
- `docs/configuration.md` - All config options documented
- Startup validation with clear error messages
- Secret rotation procedure documentation
