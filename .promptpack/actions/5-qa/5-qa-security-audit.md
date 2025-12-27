---
description: Security Audit
---

# [5-QA] Security Assessment & Remediation

> **Stage**: Quality Assurance | **Category**: Security | **Frequency**: Pre-release, periodic

Comprehensive security audit with remediation. Track in `TODO.md`.

## Audit Scope

Full codebase white-box analysis following OWASP guidelines.

## Assessment Areas

1. **Input Validation**: Injection vectors (SQL, XSS, Command, Path)
2. **Authentication/Authorization**: Session management, access controls
3. **Data Protection**: Encryption, sensitive data exposure, logging
4. **Dependencies**: Known CVEs in packages
5. **Configuration**: Secrets management, debug flags, CORS
6. **Cryptography**: Algorithm strength, key management
7. **Error Handling**: Information leakage via errors

## Tools (Use as Appropriate)

- Static analyzers (semgrep, bandit, eslint-security)
- Dependency scanners (npm audit, safety, trivy)
- Secret scanners (gitleaks, trufflehog)

## Output: `docs/security-audit-report.md`

```markdown
## Executive Summary
## Findings (per finding):
- ID, Title, Severity (CVSS 3.1 score)
- Affected component
- Description & Impact
- Proof of concept
- Remediation recommendation
## Risk Matrix
## Remediation Priority Queue
```

## Remediation Protocol

1. Fix HIGH/CRITICAL first
2. Verify fix doesn't break functionality
3. Ensure no performance degradation >5%
4. Add security regression test
5. Update security documentation
