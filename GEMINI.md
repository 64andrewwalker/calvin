# Calvin - Project Context

## Project Overview

**Calvin** is a PromptOps compiler and synchronization tool written in Rust. It serves as a "single source of truth" for AI coding assistant configurations. It compiles "promptpacks" (Markdown-based rules and commands) into platform-specific configuration files for tools like Claude Code, Cursor, GitHub Copilot, Antigravity, and Codex.

**Key Features:**
*   **Single Source of Truth:** Maintain prompts in `.promptpack/`.
*   **Multi-Platform Compilation:** Supports Claude, Cursor, VS Code, etc.
*   **Architecture:** Strict Clean Architecture (Presentation → Application → Domain ← Infrastructure).
*   **Security:** Auto-generates deny lists and blocks risky MCP servers.

## Building and Running

The project is a standard Rust binary.

### Key Commands

*   **Build (Debug):** `cargo build`
*   **Build (Release):** `cargo build --release`
*   **Test:** `cargo test` (Runs all tests, including unit and integration)
*   **Format:** `cargo fmt`
*   **Lint:** `cargo clippy` (Ensure no warnings)
*   **Run CLI:** `cargo run -- <COMMAND>` (e.g., `cargo run -- deploy`)

### Common Usage (CLI)

*   `calvin deploy`: Compiles and writes outputs to the project.
*   `calvin check`: Validates configuration and security.
*   `calvin diff`: Previews changes without writing.
*   `calvin watch`: Watches for file changes and auto-recompiles.

## Development Conventions

**Strict adherence to these conventions is required.**

### Architecture & Structure
The codebase follows **Clean Architecture** with four distinct layers:
1.  **Domain** (`src/domain/`): Pure business logic. **No I/O dependencies.** Defines traits (ports).
2.  **Application** (`src/application/`): Use cases and orchestration. Depends on Domain.
3.  **Infrastructure** (`src/infrastructure/`): Adapters, file system, external integrations. Implements Domain ports.
4.  **Presentation** (`src/presentation/`, `src/commands/`, `src/ui/`): CLI entry points and user interaction.

*   **Dependency Rule:** Dependencies only point inward. Domain depends on nothing.
*   **File Size:** Keep files under **400 lines**. Use `calvin-no-split` if a larger file is justified.

### Coding Standards
*   **TDD:** Write tests first. All features must be tested.
*   **Formatting:** Use `cargo fmt`.
*   **Paths:** Use `PathBuf` instead of strings for file paths.
*   **Error Handling:** Use `anyhow::Result` for CLI/Application layers, custom errors for Domain.
*   **Documentation:** Update `docs/` when changing behavior (Architecture, API, CLI reference).

### Key Files & Directories
*   `src/main.rs`: CLI entry point.
*   `src/lib.rs`: Library exports.
*   `src/domain/ports/`: Trait definitions (contracts).
*   `src/infrastructure/adapters/`: Platform-specific implementations.
*   `.promptpack/`: Default directory for source prompts.
*   `AGENTS.md`: Detailed developer guide (refer to this for in-depth rules).

### CI/CD
*   **CI:** GitHub Actions (`.github/workflows/ci.yml`) runs tests, coverage (Linux), and linting on Ubuntu, Windows, and macOS.
*   **Coverage:** 70% threshold required (enforced on Linux).
