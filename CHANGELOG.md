# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Features

* **skills:** add binary asset support for skill directories ([binary-skill-assets-prd](docs/proposals/binary-skill-assets-prd.md))
  - Binary files (images, PDFs, etc.) in skill directories are now detected and deployed
  - Binary files tracked in lockfile with `is_binary = true` flag
  - Orphan detection and cleanup includes binary files
  - Remote deployment support via SSH stdin binary transfer

## [0.7.1](https://github.com/64andrewwalker/calvin/compare/v0.7.0...v0.7.1) (2025-12-27)


### Bug Fixes

* **ci:** increase timeout for slow CLI tests in nextest ([5f7500d](https://github.com/64andrewwalker/calvin/commit/5f7500dc6e733b1f0fdb1d48d27634bde70b9d4b))
* **docs:** remove ES module imports from _home.mdx ([e01c8f5](https://github.com/64andrewwalker/calvin/commit/e01c8f5bbc43f330e0288586b27a9d28ffcf7f53))
* **docs:** use Platform components instead of inline HTML ([b7a036d](https://github.com/64andrewwalker/calvin/commit/b7a036db47bf4c72c9ccac9305daae1802ee3cd1))
* **tests:** exclude frontmatter delimiter from proptest yaml_lines ([e88e19a](https://github.com/64andrewwalker/calvin/commit/e88e19aee3ba0487225fddfc45838623b6531a5d))


### Performance Improvements

* skip doc tests in pre-commit hook ([86a81f0](https://github.com/64andrewwalker/calvin/commit/86a81f0210002eed2b9d0d93263ae67562ffae38))
* use cargo-nextest for faster test execution ([12f8f42](https://github.com/64andrewwalker/calvin/commit/12f8f42f61f409f5f53e53f9e02f924067ae8064))

## [0.7.0](https://github.com/64andrewwalker/calvin/compare/v0.6.0...v0.7.0) (2025-12-27)


### Features

* **ignore:** add --show-ignored and --debug-ignore CLI flags ([85ea089](https://github.com/64andrewwalker/calvin/commit/85ea089690c7d6020f3f4c777752fdeac38806a6))
* **ignore:** add .calvinignore support for layer filtering ([4a9d97e](https://github.com/64andrewwalker/calvin/commit/4a9d97e60aba94c3a3f12c4346f7008b7b4bc8e5))
* **json-output:** enforce consistent event and command fields across all JSON outputs ([4ceadf2](https://github.com/64andrewwalker/calvin/commit/4ceadf2b89ea6fc42ee856f23d0f77a61264a5e2))
* **skills:** add SKILL.md directory assets ([afc5608](https://github.com/64andrewwalker/calvin/commit/afc56084fd825cbdce3df3bd2129e03041513826))


### Bug Fixes

* Ensure docs content directory exists before clearing. ([358ff06](https://github.com/64andrewwalker/calvin/commit/358ff06fdb5cedc8561518229e5c2180851dc838))
* **skills:** harden supplemental path validation ([08b01f6](https://github.com/64andrewwalker/calvin/commit/08b01f6ed7fd0f2b9cfcce34a176a0f965c14c78))
* **skills:** reject root/prefix supplemental paths ([14c4aec](https://github.com/64andrewwalker/calvin/commit/14c4aecc5d2df2ad08146eac84aae43756d67df8))
* **skills:** surface deploy warnings via adapter validation ([d0f1c8a](https://github.com/64andrewwalker/calvin/commit/d0f1c8ad35bd9834608c29e52d8a57b99e807543))

## [0.6.0](https://github.com/64andrewwalker/calvin/compare/v0.5.1...v0.6.0) (2025-12-26)


### Features

* **clean:** delete empty lockfile after all assets cleaned ([cbfc058](https://github.com/64andrewwalker/calvin/commit/cbfc05883046d91bfcf23200d3c11648a8e202b9))
* **config:** add validation warnings for invalid environment variables ([2915a78](https://github.com/64andrewwalker/calvin/commit/2915a788fb89b4910714b48536ee6cb63b2d08fb))
* **config:** add validation warnings for invalid environment variables ([43de992](https://github.com/64andrewwalker/calvin/commit/43de992a4da3fd35bd55430ccbb3eace60b828a5))
* **deploy:** add --project override ([331e863](https://github.com/64andrewwalker/calvin/commit/331e863e41232f731ec7a8fba1720003cb1c2fa5))
* **deploy:** add interactive layer selection for multi-layer deploys ([003960f](https://github.com/64andrewwalker/calvin/commit/003960f74b924b8bcc531e92768035530eb2d45d))
* **errors:** harden registry/lockfile and add migration tests ([243d329](https://github.com/64andrewwalker/calvin/commit/243d3297454db79ac70be3fc8ab1b8d083244a07))
* **interactive:** add layer selection for deployment ([f62829f](https://github.com/64andrewwalker/calvin/commit/f62829f240febd0667e774eb0cd438de9585ee66))
* **layers:** add Layer entity ([0051a78](https://github.com/64andrewwalker/calvin/commit/0051a784532bb7ea3c39375368acbbc6eb7e8001))
* **layers:** add LayerLoader and FsLayerLoader ([479d691](https://github.com/64andrewwalker/calvin/commit/479d691801b08a03bf58543ad4dec025d3536d63))
* **layers:** add LayerResolver ([5f78847](https://github.com/64andrewwalker/calvin/commit/5f788470ae70082d0acb219472be3254875d14b5))
* **layers:** deploy from merged multi-layer stack ([395bf55](https://github.com/64andrewwalker/calvin/commit/395bf55d7b8178af7c8e17b8c0a9bd8c638caed2))
* **layers:** merge assets across layers ([2f56889](https://github.com/64andrewwalker/calvin/commit/2f56889064d9926e73249757ec4a39f7307a6c3c))
* **lockfile:** migrate to project root with provenance ([a01d360](https://github.com/64andrewwalker/calvin/commit/a01d3603e039e6e1650a887251e6b33fbc53bc68))
* **registry:** add domain Registry entity ([3ba5f79](https://github.com/64andrewwalker/calvin/commit/3ba5f790c65d30ffe3a223e5d0313c6a38a2b233))
* **registry:** add projects and clean-all commands ([d9dffc1](https://github.com/64andrewwalker/calvin/commit/d9dffc1c4fc4b473debe29e629c6f496a0e041f7))
* **registry:** add RegistryRepository port ([f789e13](https://github.com/64andrewwalker/calvin/commit/f789e13d8769cdee65c2c8181059209c96052a43))
* **registry:** add TOML registry repository ([c3a2440](https://github.com/64andrewwalker/calvin/commit/c3a2440470dd6eb12ffc34b57d9ff45a8fd01a0f))
* **registry:** register projects on deploy ([66891a9](https://github.com/64andrewwalker/calvin/commit/66891a965f5daf7772bf1d88890d91463ef26fbd))
* **scripts:** add worktree initialization scripts ([49a5666](https://github.com/64andrewwalker/calvin/commit/49a5666271fc61e86a114eadee937551e7bc9883))
* **sources:** add config + CLI layer controls ([f5b706e](https://github.com/64andrewwalker/calvin/commit/f5b706e0d27361e49e605e9af57d244fd71bf198))
* **tests:** add WindowsCompatExt trait for cross-platform test isolation ([60d01d2](https://github.com/64andrewwalker/calvin/commit/60d01d2078f20ff9352719063f86136000ab9eaf))
* **visibility:** add layers and provenance tooling ([6ab9a35](https://github.com/64andrewwalker/calvin/commit/6ab9a350e3344f3eb43b9f114a6d967311811206))
* **visibility:** enhance layer stack display with provenance and tilde paths ([96178a6](https://github.com/64andrewwalker/calvin/commit/96178a681dc17854dd68625286032afa3345d32d))


### Bug Fixes

* Add `USERPROFILE` environment variable for Windows compatibility in tests, Sensei. ([f97f7cf](https://github.com/64andrewwalker/calvin/commit/f97f7cf86e6b1541327b4b8af919fa4921c4cebf))
* centralize documentation URLs in docs module ([0cd2d40](https://github.com/64andrewwalker/calvin/commit/0cd2d40c26b41a6ba071cb4a7414b5c213da1b73))
* **clean:** interactive choose project vs home ([cdedded](https://github.com/64andrewwalker/calvin/commit/cdeddedd68b6c3ae695e36bfc66e7a1f082e5ccc))
* **config:** add CALVIN_USER_CONFIG_PATH for Windows CI compatibility ([9e48e8a](https://github.com/64andrewwalker/calvin/commit/9e48e8a5508fc2ee2f6c1f74ad5c293b2caea514))
* **config:** add CALVIN_XDG_CONFIG_PATH for Windows CI compatibility ([6a31290](https://github.com/64andrewwalker/calvin/commit/6a312909e70fd8b71d4b21c1c1bd676d31a5cbfa))
* **deploy:** respect project root and exit codes ([175bf3c](https://github.com/64andrewwalker/calvin/commit/175bf3c3ea3a0e2adc2a3d42f2bef75a5eb1850b))
* **home:** global lockfile for --home deploy/clean ([189cc23](https://github.com/64andrewwalker/calvin/commit/189cc23596d3e8be4cc62abfb70ba1ebb0107783))
* **interactive:** add global_layers_only state alias ([31a6066](https://github.com/64andrewwalker/calvin/commit/31a6066e8910a10fea43911a2bd459ce64aa9a8d))
* **interactive:** detect user layer when no project .promptpack exists ([7a71032](https://github.com/64andrewwalker/calvin/commit/7a7103229ef0704f00a5e665190da446c5b90fa5))
* **multi-layer:** align diff + targets merge with PRD ([96a48dd](https://github.com/64andrewwalker/calvin/commit/96a48dde01c3a46708b1c4e7b52cee3d8f78594a))
* **multi-layer:** interactive sources, layered promptpack config, init schema ([813a370](https://github.com/64andrewwalker/calvin/commit/813a370bdf66e1d66f9e8aedc732a851462491b5))
* **multi-layer:** project-root lockfile, layered targets, interactive menu ([9c02632](https://github.com/64andrewwalker/calvin/commit/9c02632430153f9e349ad6a59938e95247856588))
* **multi-layer:** remote ~, diff/watch project_root, config merge ([8b92b51](https://github.com/64andrewwalker/calvin/commit/8b92b5115bfc83a7a1c91cb285687cff851faeb8))
* **multi-layer:** watch stack, disable project layer, legacy config ([0e995e0](https://github.com/64andrewwalker/calvin/commit/0e995e0c946cb84b616bef80238229470f255df6))
* normalize path separators for Windows CI ([61720a7](https://github.com/64andrewwalker/calvin/commit/61720a7e56d1921e1a2f0002a63a607d172e4ef8))
* **setup:** enhance lockfile hash retrieval for cross-platform compatibility ([4bbb2cd](https://github.com/64andrewwalker/calvin/commit/4bbb2cd13d1619b05ed9d92fb0eb8a0d3a8b219c))
* **test:** escape Windows paths in TOML config for legacy user config test ([75da682](https://github.com/64andrewwalker/calvin/commit/75da6823d49fa267d18083030cc61488e957e308))
* **test:** escape Windows paths in TOML config for user_layer_path_config test ([29f546c](https://github.com/64andrewwalker/calvin/commit/29f546c0bc32b81e5fdf32afdaa56ff887fb7f95))
* **tests:** add Windows CI compatibility for directory path tests ([85149f7](https://github.com/64andrewwalker/calvin/commit/85149f73b6cb97c38cf5afae8b2c2e188135ef33))
* **tests:** handle Windows path separators in JSON output assertions ([2feb18f](https://github.com/64andrewwalker/calvin/commit/2feb18f9ade1b4a715f3d069b03c27e1bae07615))
* **tests:** use forward slashes in TOML paths for Windows compatibility ([c723cd7](https://github.com/64andrewwalker/calvin/commit/c723cd72062eb310a9963feec5ae0059d4ead8ac))
* **test:** update test configuration to enhance ci stability ([940b999](https://github.com/64andrewwalker/calvin/commit/940b9996ed685f2ecd0e896986b26d0bda436470))
* **ui:** use actual asset_count in deploy summary instead of estimate ([ba84d11](https://github.com/64andrewwalker/calvin/commit/ba84d112990f08f93a39680f0a452eb799479f3c))
* **watch:** process notify events correctly ([5b17642](https://github.com/64andrewwalker/calvin/commit/5b1764205f151569670077a881e09a150b58aaba))
* **windows:** complete home directory unification for test isolation ([bf61a22](https://github.com/64andrewwalker/calvin/commit/bf61a22827f754b2a28c9f589f97e2e44c068964))
* **windows:** unify home directory resolution with calvin_home_dir() ([22c52b5](https://github.com/64andrewwalker/calvin/commit/22c52b5a0c49cc6e09249a42bf5206b50e99d3fd))

## [Unreleased]

### Added

- **Multi-Layer System (Phase 1)**: Load and merge assets from multiple layers
  - User layer (`~/.calvin/.promptpack/`) with lowest priority
  - Additional layers via `--layer` flag (middle priority)
  - Project layer (`./.promptpack/`) with highest priority
  - Verbose mode (`-v`) displays resolved layer stack
  - Same-ID assets: higher-priority layer overrides lower
  - Different-ID assets: all are preserved
  - Symlink support with circular symlink detection
  - Lockfile now records `source_layer`, `source_file`, and `overrides` provenance

- **Global Registry (Phase 2)**: Track all Calvin-managed projects
  - `calvin projects` - List all registered projects with asset counts and last-deploy times
  - `calvin projects --prune` - Remove entries for projects with missing lockfiles
  - Successful `calvin deploy` automatically registers projects to `~/.calvin/registry.toml`
  - Paths stored as canonical absolute paths for stable matching

- **Registry-Wide Clean**: `calvin clean --all` now cleans all registered projects
  - Iterates all projects in the registry
  - Skips projects with missing lockfiles
  - JSON output includes per-project results

- **Visibility & Tooling (Phase 4)**: New inspection and debugging commands
  - `calvin layers` - Show resolved multi-layer stack with asset counts
  - `calvin provenance` - Show lockfile provenance (source layer, asset, file, overrides)
  - `calvin provenance --filter <SUBSTR>` - Filter outputs by substring
  - `calvin check --all` - Check all registered projects (global registry)
  - `calvin check --all-layers` - Check all resolved layers for the current project
  - `calvin migrate` - Migrate lockfile from legacy to new location
  - `calvin migrate --dry-run` - Preview migration without applying

### Changed

- **Breaking**: `calvin clean --all` semantic changed from "all scopes (home + project)" to "all registered projects"
  - To clean both scopes in the current project, use interactive mode or run `--home` and `--project` separately
- Lockfile location changed from `.promptpack/.calvin.lock` to `./calvin.lock` (Phase 0, auto-migrated)

### Fixed

- Relative paths (e.g., `./`) now canonicalized before registry storage

---

## [0.5.1](https://github.com/64andrewwalker/calvin/compare/v0.5.0...v0.5.1) (2025-12-24)

### Bug Fixes

- Cursor-only deploy now generates commands ([9a085e2](https://github.com/64andrewwalker/calvin/commit/9a085e2b274516e2f7c21cc8828ce33dfc322b4d))
- Cursor-only deploy now generates commands ([a57c720](https://github.com/64andrewwalker/calvin/commit/a57c7207cf3ada8353754bd5ea9da860d669c920))
- respect targets config in config.toml ([9c6420b](https://github.com/64andrewwalker/calvin/commit/9c6420b675427bc985fd2bd4bd97ef5ffef30049))

## [0.5.0](https://github.com/64andrewwalker/calvin/compare/v0.4.0...v0.5.0) (2025-12-23)

### Features

- **clean:** add --all option for non-interactive full cleanup ([ac2544a](https://github.com/64andrewwalker/calvin/commit/ac2544abab90ef116602e332020f8fe4b23a59c6))
- **clean:** add TreeMenu widget and clean UI views ([271884a](https://github.com/64andrewwalker/calvin/commit/271884a5638817ef376890a5099724e9355e3ea9))
- **clean:** filter non-cleanable files in interactive mode ([0fb523a](https://github.com/64andrewwalker/calvin/commit/0fb523a9b3d9a21b66a29f14ad6ac77fbd589ee4))
- **clean:** implement API improvements following TDD ([84c5aad](https://github.com/64andrewwalker/calvin/commit/84c5aadc2887138d0c370c17f7e398e595c50a13))
- **fuzz:** add 5 new fuzz targets for comprehensive coverage ([ac9d52d](https://github.com/64andrewwalker/calvin/commit/ac9d52d0557e914c8a16877db49a755d5192ca3f))
- implement clean command with CLI options and tests ([f4a5b83](https://github.com/64andrewwalker/calvin/commit/f4a5b837fdded5e2be6aca7c7d03012af9666e84))
- **ui:** add CalvinTheme with ●/○ icons and fix cursor restoration ([1700e4e](https://github.com/64andrewwalker/calvin/commit/1700e4e592eb303dc372f78ac6760af775930f1d))
- update GEMINI.md to a comprehensive development guide and apply minor formatting adjustments to CLAUDE.md and AGENTS.md. ([ab44248](https://github.com/64andrewwalker/calvin/commit/ab44248ff6dca1b567aa0db14a5276332e36facc))

### Bug Fixes

- **clean:** remove orphan lockfile entries for missing/unsigned files ([37437ce](https://github.com/64andrewwalker/calvin/commit/37437ce6945251bb4785b88548f07a7deae9d45c))
- **clean:** respect selected_keys for selective deletion ([3895539](https://github.com/64andrewwalker/calvin/commit/3895539a2f25b66920df68f7ea5596752103998f))
- normalize paths in CleanUseCase tests for Windows CI ([967e3c0](https://github.com/64andrewwalker/calvin/commit/967e3c0b3b84e29ef6c37244a6eea4cc7bb6640f))
- normalize paths in integration tests for Windows CI ([50faaec](https://github.com/64andrewwalker/calvin/commit/50faaecf0e387f7824cefa2280fe562f03111d32))
- remove version from generated file headers/footers ([00506a7](https://github.com/64andrewwalker/calvin/commit/00506a7fd08175c56bccfb124e619f4755c3b01c))
- **vscode:** only generate AGENTS.md for project scope deployments ([c5d8b3f](https://github.com/64andrewwalker/calvin/commit/c5d8b3f6dab86c278c92b84fa9846059a09d9596))

## [0.4.0](https://github.com/64andrewwalker/calvin/compare/v0.3.0...v0.4.0) (2025-12-22)

### Features

- add Homebrew formula and auto-update workflow ([f115bc8](https://github.com/64andrewwalker/calvin/commit/f115bc8ef8c295ce7a4f2fe59de224f7af57d828))
- **ci:** add release-please for automated changelog ([1c63183](https://github.com/64andrewwalker/calvin/commit/1c6318348277ea4db2bde3848a758d2c456a94ec))

### Bug Fixes

- **ci:** use gh CLI to download release assets for checksums ([a450e85](https://github.com/64andrewwalker/calvin/commit/a450e856dad361045fe37fe5757c7e7797d40efb))

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
