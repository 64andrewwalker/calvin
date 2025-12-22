# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0](https://github.com/64andrewwalker/calvin/compare/v0.3.0...v0.4.0) (2025-12-22)


### Features

* add Homebrew formula and auto-update workflow ([f115bc8](https://github.com/64andrewwalker/calvin/commit/f115bc8ef8c295ce7a4f2fe59de224f7af57d828))
* **ci:** add release-please for automated changelog ([1c63183](https://github.com/64andrewwalker/calvin/commit/1c6318348277ea4db2bde3848a758d2c456a94ec))


### Bug Fixes

* **ci:** use gh CLI to download release assets for checksums ([a450e85](https://github.com/64andrewwalker/calvin/commit/a450e856dad361045fe37fe5757c7e7797d40efb))

## [0.3.0] - 2025-12-23

### Added

- **Clean Architecture (v2)**: Complete four-layer architecture ✅ verified
  - `domain/` - Business logic (entities, value objects, services, policies)
  - `application/` - Use cases (DeployUseCase, CheckUseCase, WatchUseCase, DiffUseCase)
  - `infrastructure/` - External integrations (adapters, repositories, file systems)
  - `presentation/` - CLI output and formatting
- **Domain Layer Components** ✅ verified
  - `Asset`, `OutputFile`, `Lockfile` entities
  - `Scope`, `Target`, `ContentHash`, `SafePath` value objects
  - `Compiler`, `Planner`, `Differ`, `OrphanDetector` services
  - `ScopePolicy`, `SecurityPolicy` policies
- **Infrastructure Adapters** ✅ verified
  - All 5 platform adapters rewritten: ClaudeCode, Cursor, VSCode, Antigravity, Codex
  - `LocalFs`, `RemoteFs` file system implementations
  - `TomlLockfileRepository`, `TomlConfigRepository` implementations
- **Remote Deployment** ✅ verified
  - SSH-based remote sync with `--remote user@host:/path`
  - rsync acceleration for efficient file transfers
  - Batch file checking for reduced SSH round-trips
- **Watch Mode E2E Tests** ✅ verified
  - Comprehensive tests in `tests/cli_watch.rs`
  - Debouncing and batching validation
- **File Size Health Check** ✅ verified
  - Pre-commit script: `scripts/check-file-size.sh`
  - Enforces <500 lines per file with documented exceptions
  - `calvin-no-split` marker for exempt files
- **Code Coverage Integration** ✅ verified
  - cargo-llvm-cov integration
  - Codecov reporting in CI
  - 75%+ line coverage maintained

### Changed

- **Dependencies Updated** ✅ verified
  - `crossterm` 0.28 → 0.29 (unifies rustix dependency)
  - `dialoguer` 0.11 → 0.12 (removes thiserror v1 duplicate)
  - `insta` 1.44 → 1.45 (latest snapshot testing)
  - `serde_yaml` → `serde_yml` (actively maintained fork)
  - `atty` → `is-terminal` (modern replacement)
- **Architecture Refactoring** ✅ verified
  - Deleted legacy `sync/` module (now compatibility layer only)
  - Split large files into module folders (watcher/, deploy/, etc.)
  - 204 files changed, +24,646/-12,163 lines

### Fixed

- **Windows CI Compatibility** ✅ verified
  - Cross-platform path handling in tests
  - Normalized source paths in footer comments (`\` → `/`)
  - Platform-specific absolute path detection in `SafePath`
- **Orphan Detection** ✅ verified
  - Check orphan file status before deletion
  - Path field added to `OrphanFile` for display

### Security

- **Security Audit Report** ✅ verified
  - Full white-box source code analysis
  - Dependency CVE scan with cargo-audit
  - `SafePath` value object for path traversal protection
  - `quote_path()` for SSH command injection prevention

### Documentation

- **Reorganized Documentation Structure** ✅ verified
  - `docs/guides/` - Developer guides
  - `docs/reports/` - Audit reports
  - `docs/archive/` - Historical documents
  - 21 historical docs moved to archive
- **Reports Added** ✅ verified
  - `security-audit-report.md` - Updated 2025-12-20
  - `dependency-report-2025-12-20.md` - Full dependency inventory
  - `cleanup-report-2025-12-20.md` - Codebase health check

---

## [0.2.0] - 2025-12-17

### Added

- Initial public release
- Multi-platform compilation (ClaudeCode, Cursor, VSCode, Antigravity, Codex)
- Security by default (auto-generated deny lists)
- Watch mode for continuous deployment
- Check command for configuration validation
- User-scope installations (`--home` flag)

---

## Statistics (vs main)

| Metric | Value |
|--------|-------|
| Commits | 100+ |
| Files Changed | 204 |
| Lines Added | 24,646 |
| Lines Removed | 12,163 |
| Tests | 600+ |
| Coverage | 75%+ |
