# TDD Session: v0.2.0 Phase 3 (Error enhancements)

## Requirements covered

- Standardized fatal error output format:
  - `[ERROR] <title>`
  - optional location line
  - body/context
  - `FIX:` suggestion (when available)
- Added “Press Enter to open in editor” flow for common file-based errors when running interactively.
- Ensured `--json` mode remains non-interactive and produces structured error output.

## RED phase (tests added)

- Added failing test asserting formatted errors include `[ERROR]` and `FIX:`:
  - `src/ui/error.rs` → `test_format_missing_field_includes_error_header_and_fix`

## GREEN phase (implementation)

- Implemented `src/ui/error.rs`:
  - `format_calvin_error()` renders `CalvinError` variants with FIX suggestions.
  - `print_error()` prints human output to stderr, or JSON to stdout when `--json` is used.
  - `offer_open_in_editor()` prompts to open the relevant file via `$VISUAL`/`$EDITOR` in TTY sessions.
- Wired the CLI to use this renderer:
  - `src/main.rs` catches command errors, prints formatted output, optionally offers editor open, and exits with code 1.

## REFACTOR notes

- Kept error formatting in `ui/` (CLI concern) without changing library error strings.
- Open-in-editor uses best-effort heuristics for popular editors (`code`, `cursor`, `vim` family).

## Validation

- `cargo test` passes.
- `cargo clippy` is clean.

