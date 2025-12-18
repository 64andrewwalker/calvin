# TDD Session Log: CLI Animation (v0.3.0)

Date: 2025-12-17  
Scope: `docs/cli-animation/*` (Phase 0 → Phase 3)  
Mode: RED → GREEN → REFACTOR, iterating until checklist completion.

## Requirements (source of truth)

- `docs/cli-animation/TODO.md`
- `docs/cli-animation/design-principles.md`
- `docs/cli-animation/ui-components-spec.md`

## Iterations

### Iteration 0 — Session setup

- Requirements covered
  - Create an auditable TDD session log (this file).
- Tests written
  - None.
- Implementation decisions
  - Drive refactor via unit tests for pure renderers (string output) + minimal integration tests for CLI output behavior in non-TTY.
- Refactoring notes
  - None.

### Iteration 1 — Terminal capability detection (Phase 0.2)

- Requirements covered
  - `src/ui/terminal.rs`: `TerminalCapabilities` + `detect_capabilities()` with NO_COLOR / CI / TERM=dumb detection.
- Tests written
  - `src/ui/terminal.rs`: 4 unit tests for NO_COLOR, CI detection, TERM=dumb downgrade, TERM 256color detection.
- Implementation decisions
  - Keep a pure `detect_capabilities_impl(get_env, is_tty, size)` to avoid mutating global env vars in tests.
- Refactoring notes
  - Fixed a pre-existing `--force/--yes` overwrite bug: sync skipped writes when lockfile hash matched output even if the file was user-modified (`src/sync/mod.rs`).

### Iteration 2 — Output config + CLI flags (Phase 0.1/0.3)

- Requirements covered
  - Add dependencies: `crossterm`, `owo-colors`, `unicode-width` (`Cargo.toml`).
  - Extend config: `[output] color|animation|unicode` (`src/config.rs`).
  - Add CLI flags: `--color`, `--no-animation` (`src/cli.rs`).
- Tests written
  - `src/config.rs`: default + TOML parsing for new output config.
  - `src/cli.rs`: parsing for `--color never` and `--no-animation`.
- Implementation decisions
  - Keep CLI `--color` as `Option` to avoid overriding config defaults when the flag is absent.
  - Keep config defaults aligned with docs: `color=auto`, `animation=auto`, `unicode=true`.
- Refactoring notes
  - None.

### Iteration 3 — Theme + render engine scaffold (Phase 0.4/0.5)

- Requirements covered
  - `src/ui/theme.rs`: unified design tokens (5 colors, icons, borders) + ASCII fallbacks.
  - `src/ui/render.rs`: minimal buffered renderer (`TerminalState`) with `clear_line`, `move_cursor`, `flush`.
- Tests written
  - `src/ui/render.rs`: buffer flush clears pending output.
- Implementation decisions
  - Use `crossterm` command queueing into an internal `Vec<u8>` buffer for testability and later live-update rendering.
- Refactoring notes
  - None.

### Iteration 4 — Core widgets (Phase 1.1–1.3, partial)

- Requirements covered
  - Primitives: `ColoredText`, `Icon`, `BorderChar` (`src/ui/primitives/*`).
  - Widgets: `Spinner`, `ProgressBar`, `StatusList`, `Box` (`src/ui/widgets/*`).
- Tests written
  - `src/ui/widgets/spinner.rs`: 3 tests (unicode frames, ASCII fallback, tick advances).
  - `src/ui/widgets/progress.rs`: 3 tests (render includes counts/percent, ETA computed, ETA none on zero progress).
  - `src/ui/widgets/list.rs`: 3 tests (pending default, status update, visible window).
  - `src/ui/primitives/icon.rs`: 2 tests (unicode vs ASCII icon selection).
- Implementation decisions
  - Keep widgets as pure string renderers with explicit `supports_unicode`/`supports_color` inputs for deterministic tests and future renderer integration.
- Refactoring notes
  - None.

### Iteration 5 — Streaming integration (Phase 1.3–1.5, partial)

- Requirements covered
  - Sync engine emits per-file events (`src/sync/mod.rs`: `SyncEvent`, `sync_*_with_callback`).
  - Deploy uses live-updating `StatusList` + `ProgressBar` when animations are enabled (`src/commands/deploy.rs`).
  - Check uses streaming callbacks while `run_doctor` populates checks (`src/security.rs`: `run_doctor_with_callback`; `src/commands/check.rs`).
- Tests written
  - `src/sync/tests.rs`: callback emits `ItemWritten` events.
- Implementation decisions
  - Keep streaming UI best-effort and only enabled for TTY + non-interactive runs (avoids interfering with conflict prompts).
- Refactoring notes
  - Introduced `UiContext` (`src/ui/context.rs`) + `LiveRegion` (`src/ui/live_region.rs`) to coordinate capability-based rendering.

### Iteration 6 — ErrorBox rendering polish (Phase 2.1–2.4, partial)

- Requirements covered
  - `ErrorBlock` uses a dedicated pointer marker (`↑`) for highlighted code lines with ASCII fallback.
  - `Box` supports multiline content safely (no broken borders when strings contain `\n`).
  - `ui/output.rs` warnings use theme icons + blocks (allow_naked warning) and keep outputs deterministic via format helpers.
  - `ui/error.rs` now prints without double-newlines and includes code context when the source file exists.
- Tests written
  - `src/ui/widgets/box.rs`: multiline line splitting.
  - `src/ui/blocks/error.rs`: unicode pointer + ASCII pointer fallback.
  - `src/ui/error.rs`: MissingField includes code context when file exists.
  - `src/ui/output.rs`: allow_naked warning includes actionable key; config warning includes suggestion line.
- Implementation decisions
  - Treat widgets/blocks as renderers that can accept multiline strings (handled in `Box::add_line`).
  - Add a dedicated `Icon::Pointer` token rather than overloading `↳` (reserved for “sub info”).
- Refactoring notes
  - Updated `deploy`/`diff` to pass `UiContext` into config warning printers (preparing for full view refactors).

### Iteration 7 — Views + diff/watch output refactor (Phase 3, partial)

- Requirements covered
  - Added `blocks/header.rs` (`CommandHeader`) and `blocks/summary.rs` (`ResultSummary`).
  - Added `views/*` and refactored `deploy`, `check`, `diff`, `watch`, and interactive banner to render via UI views/blocks (reduced direct formatting in commands).
  - Added diff rendering component with per-line numbers + color mapping (`ui/components/diff.rs`).
- Tests written
  - `src/ui/blocks/header.rs`: header renders ASCII icon fallback.
  - `src/ui/blocks/check_item.rs`: recommendation line uses arrow icon in ASCII mode.
  - `src/ui/views/check.rs`: platform header appears (ensures “Codex” is still visible in CLI output).
  - `src/ui/views/diff.rs`: diff header uses diff icon in ASCII mode.
  - `src/ui/components/diff.rs`: insert/delete prefixes rendered.
  - `src/ui/views/watch.rs`: started event uses watch icon in ASCII mode.
- Implementation decisions
  - Keep command logic (parsing/syncing/watching) in `commands/*` but move string shaping into `ui/views/*`.
  - Diff output uses `similar::TextDiff` and a line-numbered unified style for readability.
- Refactoring notes
  - `deploy` now ends with a `ResultSummary` box instead of ad-hoc “Results” prints.
  - `check` report + summary are now rendered through `ui/views/check`.

### Iteration 8 — NDJSON event streams (Phase 3.5, partial)

- Requirements covered
  - `--json` now emits NDJSON (one JSON object per line) with `start`/`complete` framing for `deploy` and `check`.
  - `deploy --json` streams per-file sync events (`item_*` + `progress`) via `SyncEvent` callbacks.
  - `check --json` streams per-check events via `run_doctor_with_callback`.
  - `diff`, `parse`, `version`, `migrate`, `explain`, and `interactive` also emit NDJSON events.
  - `WatchEvent::to_json()` now uses `serde_json` serialization (correct escaping).
- Tests written
  - `tests/cli_json_deploy.rs`: asserts NDJSON stream with `start`/`complete` + `item_written`.
  - `tests/cli_json_check.rs`: asserts NDJSON stream with `start`/`complete` + at least one `check` event.
- Implementation decisions
  - Add `ui/json.rs` as a tiny NDJSON writer (`write_event`/`emit`) and reuse it across commands.
  - Keep JSON events compact (no pretty printing) to preserve 1-line-per-event semantics.
- Refactoring notes
  - `ui/output.rs` emits config warnings as NDJSON `warning` events in `--json` mode.

### Iteration 9 — CI overrides + Unicode fallbacks (AP-3/AP-5, Phase 0.2/2.6)

- Requirements covered
  - CI mode disables animations even when config is `always`.
  - CI mode defaults to `--color never` behavior when `color=auto` (but respects explicit `--color always`).
  - Progress rendering supports Unicode/ASCII fallbacks across styles.
- Tests written
  - `src/ui/context.rs`: CI forces `animation=false`; CI disables `color` when auto; explicit color override still enables color.
  - `src/ui/widgets/progress.rs`: ASCII fallback for `Bar`; Unicode/ASCII rendering for `Blocks`; `Compact` omits the bar.
- Implementation decisions
  - Add `UiContext::from_caps(...)` for deterministic capability-driven tests.
  - Keep widgets as pure renderers: `ProgressBar::render(supports_unicode)` takes an explicit capability flag.
- Refactoring notes
  - None.

### Iteration 10 — Enforce theme tokens + remove hardcoded UI glyphs (DC-1/DC-2/DC-3)

- Requirements covered
  - No hardcoded UI icons/emoji outside `src/ui/theme.rs`.
  - Theme palette (5 colors) used consistently via semantic helpers/components.
- Tests written
  - `tests/no_hardcoded_ui_tokens.rs`: fails when UI glyphs (✓ ⚠ ↳ 📦 🔍 etc.) appear outside `src/ui/theme.rs`.
- Implementation decisions
  - Enforce “design tokens only” via a repository scan over `src/commands`, `src/ui`, and `src/sync`.
  - Replace remaining direct glyph/emoji prints with `ui::primitives::icon::Icon` and `theme` tokens.
- Refactoring notes
  - Replace ad-hoc styling with `crossterm::style::Stylize` + `theme::colors::*` in key UI modules.
  - Remove/trim unused UI helpers and resolve clippy warnings introduced by the refactor.

### Iteration 11 — Clippy all-targets cleanup + release build verification (Phase 0.6)

- Requirements covered
  - `cargo clippy --all-targets --all-features` passes with zero warnings.
  - `cargo build --release` succeeds (current `target/release/calvin` size: ~1.1MB).
- Tests written
  - None (lint-driven iteration).
- Implementation decisions
  - Replace owned `PathBuf::from(...)` comparisons in tests with borrowed `Path::new(...)`.
  - Add `Default` for `MockFileSystem` to match `new()` presence.
  - Remove unnecessary `clone()` on `Copy` targets in golden tests.
- Refactoring notes
  - None.

### Iteration 12 — Remove legacy borders + remote transfer progress (DC-1, Phase 1.5/3.1 partial)

- Requirements covered
  - Interactive setup wizard no longer uses legacy `=====` separators; all headings use themed `Box` borders.
  - `version`, `migrate`, `parse` non-JSON output migrated to themed boxed UI (no legacy `┌─/└─`).
  - Remote deploy shows detailed transfer feedback (file sizes + progress + speed/ETA) when animations are enabled.
- Tests written
  - `tests/no_legacy_borders.rs`: forbids legacy border tokens in `src/commands` + `src/ui`.
  - `tests/cli_version.rs`, `tests/cli_migrate.rs`, `tests/cli_parse.rs`: assert boxed output (themed borders) and remove legacy box-drawing chars.
  - `src/ui/views/transfer.rs`: bytes + transfer stats formatting tests.
  - `src/ui/widgets/progress.rs`: can hide ETA when rendered (used for transfer stats line).
- Implementation decisions
  - Add `ui/views/transfer.rs` helpers (`format_bytes_compact`, `render_transfer_stats`) for reusable transfer UI.
  - Prefer SSH-based sync for remote deploy when JSON/events or TTY animations are requested; keep rsync as a fast path when structured progress is not needed.
- Refactoring notes
  - `interactive.rs` step/section headings now render via `ui/views/interactive.rs` helpers.
  - `commands/debug.rs` non-JSON output now renders through `ui/views/{version,migrate,parse}.rs`.
