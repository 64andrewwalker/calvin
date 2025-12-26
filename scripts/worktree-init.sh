#!/usr/bin/env bash
#
# worktree-init.sh - Initialize a Git worktree environment
#
# This script is the single entry point for worktree initialization.
# It is idempotent - safe to run multiple times.
#
# Responsibilities:
#   1. Verify we're in a valid Git worktree (not main checkout)
#   2. Check if initialization is needed (via marker file)
#   3. Delegate to scripts/setup.sh for actual project setup
#
# Usage:
#   ./scripts/worktree-init.sh           # Normal init
#   FORCE_INIT=1 ./scripts/worktree-init.sh  # Force re-init
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MARKER_FILE="$REPO_ROOT/.worktree-initialized"
SETUP_SCRIPT="$SCRIPT_DIR/setup.sh"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
DIM='\033[2m'
NC='\033[0m'

info() { echo -e "${BLUE}ℹ${NC} $*"; }
success() { echo -e "${GREEN}✓${NC} $*"; }
warn() { echo -e "${YELLOW}⚠${NC} $*"; }
error() { echo -e "${RED}✗${NC} $*" >&2; }
dim() { echo -e "${DIM}$*${NC}"; }

# Step 1: Check if we're inside a Git worktree
# Uses Git commands directly, not path conventions
is_in_worktree() {
    # First, verify we're in a Git-managed directory
    if ! git rev-parse --is-inside-work-tree > /dev/null 2>&1; then
        return 1
    fi
    
    local current_path
    current_path="$(pwd -P)"
    
    # Parse git worktree list to find if current path is a worktree
    # Main checkout has a different structure than worktrees
    while IFS= read -r line; do
        if [[ "$line" == worktree\ * ]]; then
            local wt_path="${line#worktree }"
            # Normalize path for comparison
            wt_path="$(cd "$wt_path" 2>/dev/null && pwd -P)" || continue
            
            if [[ "$current_path" == "$wt_path" ]]; then
                # Found matching worktree, check if it's a linked worktree
                # Main worktree has .git directory, linked worktrees have .git file
                if [[ -f "$current_path/.git" ]]; then
                    return 0  # This is a linked worktree
                fi
            fi
        fi
    done < <(git worktree list --porcelain 2>/dev/null)
    
    return 1  # Not a linked worktree (might be main checkout)
}

# Step 2: Check if initialization is needed
# Only checks marker file - doesn't try to infer from build artifacts
needs_init() {
    # Force init if requested
    if [[ "${FORCE_INIT:-}" == "1" ]]; then
        return 0
    fi
    
    # No marker file = needs init
    if [[ ! -f "$MARKER_FILE" ]]; then
        return 0
    fi
    
    return 1
}

# Step 3: Run project setup
run_setup() {
    if [[ -x "$SETUP_SCRIPT" ]]; then
        info "Running project setup..."
        "$SETUP_SCRIPT"
    else
        error "Setup script not found or not executable: $SETUP_SCRIPT"
        echo ""
        echo "Create the setup script or make it executable:"
        echo "  chmod +x $SETUP_SCRIPT"
        return 1
    fi
}

# Save initialization state
mark_initialized() {
    date -u +"%Y-%m-%dT%H:%M:%SZ" > "$MARKER_FILE"
}

main() {
    cd "$REPO_ROOT"
    
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo " Worktree Initialization"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    
    # Step 1: Verify we're in a worktree
    if ! is_in_worktree; then
        warn "Not running in a linked worktree"
        dim "  This appears to be the main checkout or a regular clone."
        dim "  Set SKIP_WORKTREE_CHECK=1 to bypass this check."
        echo ""
        
        if [[ "${SKIP_WORKTREE_CHECK:-}" != "1" ]]; then
            echo "If you want to run setup anyway:"
            echo "  SKIP_WORKTREE_CHECK=1 $0"
            exit 0
        fi
    fi
    
    # Show context
    local current_branch
    current_branch="$(git branch --show-current 2>/dev/null || echo 'detached')"
    info "Branch: $current_branch"
    info "Path: $REPO_ROOT"
    echo ""
    
    # Step 2: Check if init is needed
    if ! needs_init; then
        success "Already initialized"
        dim "  Last init: $(cat "$MARKER_FILE" 2>/dev/null || echo 'unknown')"
        dim "  To force re-init: FORCE_INIT=1 $0"
        echo ""
        exit 0
    fi
    
    # Step 3: Run setup (delegates to scripts/setup.sh)
    if run_setup; then
        mark_initialized
        echo ""
        success "Initialization complete!"
        echo ""
    else
        error "Initialization failed"
        echo ""
        echo "Fix the errors above and run again:"
        echo "  FORCE_INIT=1 $0"
        exit 1
    fi
}

main "$@"
