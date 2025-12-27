---
description: Git Audit
---

# [M-MAINT] Git Repository Audit & Cleanup

> **Stage**: Maintenance | **Category**: Repository Hygiene | **Frequency**: Periodic

Comprehensive Git hygiene review. Track in `TODO.md`.

## Audit Areas

1. **Branch Analysis**:
   - Stale branches (no commits >30 days)
   - Merged but undeleted branches
   - Orphaned branches
2. **Worktree Status**: List and verify all worktrees
3. **Stash Review**: Old stashes (>7 days), potential data loss
4. **Large Files**: Files >1MB, binaries that should be compressed
5. **Sensitive Data Leakage**:
   - Scan for secrets in history (API keys, passwords, tokens)
   - Credentials in config files
   - Private keys, certificates

## .gitignore Verification

Ensure exclusion of:

- IDE configs (`.idea/`, `.vscode/`)
- Agent workspaces (`.cursor/`, `.claude/`, `.antigravity/`)
- Environment files (`.env*`)
- Build artifacts
- OS files (`.DS_Store`, `Thumbs.db`)

## Cleanup Actions

1. Delete merged/stale branches: `git branch -d <branch>`
2. Prune worktrees: `git worktree prune`
3. Clear old stashes (with user confirmation)
4. Compress binaries: `ffmpeg`, `pngquant`, `jpegoptim`
5. **If secrets found in history**:
   - Use `git filter-repo` to purge from all commits
   - Alert user to rotate compromised credentials
   - Force push required (document impact)

## Output: `docs/git-audit-report.md`

- Repository health score
- Action items with commands
- Leakage findings (CRITICAL)
- Size optimization opportunities
