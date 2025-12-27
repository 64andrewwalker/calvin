---
description: Internationalization Setup
---

# [2-INIT] Internationalization Setup

> **Stage**: Initialize | **Category**: Localization | **Frequency**: Multi-language support

Implement or audit internationalization infrastructure. Track in `TODO.md`.

## Audit (If Existing)

1. **String Extraction**: Find hardcoded user-facing strings
2. **Format Review**: Date, number, currency, pluralization handling
3. **RTL Support**: Layout compatibility for RTL languages
4. **Coverage**: Missing translations per locale

## Implementation

1. **Framework Setup**:
   - Install i18n library (i18next, react-intl, vue-i18n, etc.)
   - Configure locale detection (URL, cookie, browser)
   - Set up fallback chain
2. **String Extraction**:
   - Extract all user-facing strings
   - Generate translation keys (namespaced)
   - Create source locale file
3. **Code Transformation**:
   - Replace hardcoded strings with t() calls
   - Implement interpolation for dynamic content
   - Handle pluralization rules
4. **Asset Localization**:
   - Images with text
   - PDFs and documents
   - Email templates

## Directory Structure

```text
locales/
├── en/
│   ├── common.json
│   ├── errors.json
│   └── <feature>.json
├── zh-CN/
└── ...
```

## Output

- Configured i18n setup
- Extracted translation files
- `docs/i18n-guide.md` for translators
- CI check for missing translations
