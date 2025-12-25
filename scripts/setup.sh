#!/usr/bin/env bash
#
# setup.sh - Project-specific setup script
#
# This script handles all project-specific initialization:
#   - Toolchain verification
#   - Dependency installation
#   - Build verification
#
# It implements "fast path" vs "slow path" logic:
#   - Fast path: Skip if dependencies are already installed and up-to-date
#   - Slow path: Full rebuild when lockfile changes or artifacts missing
#
# Called by: scripts/worktree-init.sh
# Can also be run directly: ./scripts/setup.sh
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOCK_HASH_FILE="$REPO_ROOT/.cargo-lock-hash"

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

# Check if Rust toolchain is available
check_toolchain() {
    info "Checking toolchain..."
    
    if ! command -v cargo &> /dev/null; then
        error "Cargo not found"
        echo ""
        echo "Install Rust toolchain:"
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        return 1
    fi
    
    dim "  Rust: $(rustc --version 2>/dev/null || echo 'unknown')"
    dim "  Cargo: $(cargo --version 2>/dev/null || echo 'unknown')"
    success "Toolchain OK"
}

# Get hash of Cargo.lock for change detection
get_lock_hash() {
    if [[ -f "$REPO_ROOT/Cargo.lock" ]]; then
        sha256sum "$REPO_ROOT/Cargo.lock" 2>/dev/null | cut -d' ' -f1
    else
        echo "no-lockfile"
    fi
}

# Check if we need to rebuild (fast path vs slow path)
needs_rebuild() {
    # No target directory = definitely need build
    if [[ ! -d "$REPO_ROOT/target" ]]; then
        return 0
    fi
    
    # No hash file = need build
    if [[ ! -f "$LOCK_HASH_FILE" ]]; then
        return 0
    fi
    
    # Compare lockfile hash
    local current_hash saved_hash
    current_hash="$(get_lock_hash)"
    saved_hash="$(cat "$LOCK_HASH_FILE" 2>/dev/null || echo "")"
    
    if [[ "$current_hash" != "$saved_hash" ]]; then
        info "Cargo.lock has changed, rebuild needed"
        return 0
    fi
    
    # Check if binary exists
    if [[ ! -f "$REPO_ROOT/target/debug/calvin" ]]; then
        return 0
    fi
    
    return 1
}

# Save lockfile hash for future comparison
save_lock_hash() {
    get_lock_hash > "$LOCK_HASH_FILE"
}

# Build the project
build_project() {
    info "Building project..."
    
    if cargo build; then
        success "Build complete"
        save_lock_hash
    else
        error "Build failed"
        return 1
    fi
}

# Quick test compilation check (optional)
check_tests() {
    info "Checking test compilation..."
    
    if cargo test --no-run 2>/dev/null; then
        success "Tests compile OK"
    else
        warn "Test compilation had issues (non-fatal)"
    fi
}

main() {
    cd "$REPO_ROOT"
    
    echo ""
    info "Calvin project setup"
    echo ""
    
    # Step 1: Verify toolchain
    if ! check_toolchain; then
        exit 1
    fi
    echo ""
    
    # Step 2: Check if build is needed (fast path)
    if ! needs_rebuild; then
        success "Dependencies up-to-date (fast path)"
        dim "  Skipping rebuild - Cargo.lock unchanged"
        echo ""
        return 0
    fi
    
    # Step 3: Build (slow path)
    if ! build_project; then
        exit 1
    fi
    echo ""
    
    # Step 4: Optional test check
    check_tests
    echo ""
    
    success "Setup complete!"
}

main "$@"

