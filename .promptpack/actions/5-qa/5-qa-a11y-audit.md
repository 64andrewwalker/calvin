---
description: Accessibility Audit
---

# [5-QA] Accessibility Audit (WCAG 2.1 AA)

> **Stage**: Quality Assurance | **Category**: Compliance | **Frequency**: Pre-release

Comprehensive accessibility review and remediation. Track in `TODO.md`.

## Automated Testing

1. Run axe-core / WAVE / Lighthouse accessibility audits
2. Validate HTML semantics
3. Check color contrast ratios
4. Verify focus management

## Manual Testing Checklist

1. **Keyboard Navigation**:
   - All interactive elements focusable
   - Logical tab order
   - No keyboard traps
   - Skip links present
2. **Screen Reader Compatibility**:
   - Meaningful alt text for images
   - ARIA labels and roles
   - Live region announcements
   - Form label associations
3. **Visual Design**:
   - Color contrast (4.5:1 text, 3:1 UI)
   - Text resizing to 200%
   - Motion preferences respected
   - Focus indicators visible
4. **Content Structure**:
   - Heading hierarchy (h1-h6)
   - Landmark regions
   - Data table headers
   - Link purpose clarity

## Output: `docs/accessibility-audit.md`

```text
## Compliance Score: X/100
## Critical Issues (WCAG Level A)
## Major Issues (WCAG Level AA)
## Minor Issues (Best Practices)
## Component-by-Component Findings
## Remediation Priority Queue
## Testing Commands for Verification
```

## Remediation

- Fix critical issues first
- Add accessibility tests to CI
- Update component library with ARIA patterns
- Create accessibility documentation for contributors
