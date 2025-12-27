---
description: Dependency Audit
---
# [5-QA] Dependency Audit

> **Stage**: Quality Assurance | **Category**: Security | **Frequency**: Periodic

Analyze and optimize project dependencies. Track in `TODO.md`.

## Tasks

1. **Inventory**: List all direct and transitive dependencies
2. **Vulnerability Scan**: Check for known CVEs
3. **License Compliance**: Identify copyleft/restrictive licenses
4. **Update Assessment**: List outdated packages with breaking change risk
5. **Unused Detection**: Find dependencies not referenced in code
6. **Duplicate Detection**: Identify multiple versions of same package

## Output

- `docs/dependency-report.md`
- Recommended `package.json` / `requirements.txt` / `Cargo.toml` updates
- License compatibility matrix
