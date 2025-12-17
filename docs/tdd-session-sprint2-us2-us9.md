# TDD Session: Sprint 2 (US-2 + US-9)

> **Started**: 2025-12-17  
> **Status**: ✅ Complete (`cargo test` passing)

## Requirements Covered

From `docs/ux-review.md`:
- [x] US-2: `security.mcp.additional_allowlist` allows internal MCP servers without doctor false positives
- [x] US-2: `calvin doctor -v` shows which MCP servers matched built-in vs custom allowlist
- [x] US-9: YAML frontmatter syntax errors include a line number + actionable hint
- [x] US-9: unknown config keys surface a warning + “did you mean …” suggestion

From `docs/cli-state-machine.md`:
- [x] Global flags behave globally (`--json`, `-v`) even when placed after subcommands
- [x] Parse errors fail sync pipeline (no silent skip)

## Tests Written

- `src/security.rs`
  - `test_mcp_allowlist_additional_allowlist_allows_unknown_server`
- `src/config.rs`
  - `test_config_parse_security_mcp_additional_allowlist`
  - `test_config_load_with_warnings_reports_unknown_key_with_suggestion`
- `src/main.rs`
  - `test_cli_json_flag_after_subcommand`
  - `test_cli_verbose_flag_after_subcommand`
- `src/parser.rs`
  - `test_parse_frontmatter_invalid_yaml_with_colon_includes_hint`
  - `test_parse_directory_fails_on_invalid_file`

## Implementation Decisions

1. **MCP allowlist configuration**
   - Added `[security.mcp] allowlist` and `additional_allowlist` to project config.
   - `doctor` accepts `.cursor/mcp.json` using either `servers` or `mcpServers` key.

2. **Verbose output**
   - Added `details: Vec<String>` to `SecurityCheck`; CLI prints details when `-v` is set.

3. **Config warnings**
   - Implemented `Config::load_with_warnings()` using `serde_ignored` to capture unknown keys.
   - Added a small Levenshtein-based suggester for “did you mean …”.

4. **Parser strictness**
   - `parse_directory()` now fails fast on invalid files (no silent skip).

5. **Frontmatter error formatting**
   - YAML parse errors are reformatted to include `Line N`, a heuristic hint for common `:` quoting mistakes, and a docs link.

## Refactoring Notes

- Global CLI flags (`--json`, `-v`) are marked `global = true`; `version --json` now uses the global flag rather than a subcommand-specific one.
- Shared config warning printing is centralized in `print_config_warnings()` in `src/main.rs`.

