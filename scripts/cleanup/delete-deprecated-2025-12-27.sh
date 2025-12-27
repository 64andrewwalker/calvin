#!/usr/bin/env bash
set -euo pipefail

# Permanently delete soft-deprecated files from the 2025-12-27 cleanup pass.
#
# Usage:
#   ./scripts/cleanup/delete-deprecated-2025-12-27.sh        # dry run
#   ./scripts/cleanup/delete-deprecated-2025-12-27.sh --yes  # delete

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

if [[ ! -f "Cargo.toml" ]]; then
  echo "error: expected to run from repo root (missing Cargo.toml)" >&2
  exit 1
fi

FILES=(
  "_deprecated/src/ui/render.rs"
  "_deprecated/src/ui/blocks/warning.rs"
)

echo "This script permanently deletes soft-deprecated files created on 2025-12-27."
echo ""
echo "Planned deletions:"
printf ' - %s\n' "${FILES[@]}"

if [[ "${1:-}" != "--yes" ]]; then
  echo ""
  echo "Dry run only. Re-run with --yes to delete."
  exit 0
fi

for path in "${FILES[@]}"; do
  rm -f -- "$path"
done

# Best-effort directory pruning (ignore failures if non-empty).
rmdir -- "_deprecated/src/ui/blocks" 2>/dev/null || true
rmdir -- "_deprecated/src/ui" 2>/dev/null || true
rmdir -- "_deprecated/src" 2>/dev/null || true
rmdir -- "_deprecated" 2>/dev/null || true

echo ""
echo "Done."

