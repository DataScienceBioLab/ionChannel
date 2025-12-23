#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# ionChannel upstream repository setup script

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
UPSTREAM_DIR="$PROJECT_ROOT/upstream"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Repository definitions
declare -A REPOS=(
    ["cosmic-comp"]="https://github.com/pop-os/cosmic-comp.git"
    ["xdg-desktop-portal-cosmic"]="https://github.com/pop-os/xdg-desktop-portal-cosmic.git"
    ["cosmic-greeter"]="https://github.com/pop-os/cosmic-greeter.git"
    ["rustdesk"]="https://github.com/rustdesk/rustdesk.git"
)

clone_or_update() {
    local name="$1"
    local url="$2"
    local target="$UPSTREAM_DIR/$name"

    if [[ -d "$target" ]]; then
        log_info "Updating $name..."
        (cd "$target" && git fetch --depth 1 origin && git reset --hard origin/HEAD)
        log_success "Updated $name"
    else
        log_info "Cloning $name..."
        git clone --depth 1 "$url" "$target"
        log_success "Cloned $name"
    fi
}

main() {
    echo ""
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║          ionChannel Upstream Repository Setup                 ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo ""

    # Check for git
    if ! command -v git &> /dev/null; then
        log_error "git is required but not installed"
        exit 1
    fi

    # Create upstream directory
    mkdir -p "$UPSTREAM_DIR"
    log_info "Upstream directory: $UPSTREAM_DIR"
    echo ""

    # Parse arguments
    local repos_to_clone=()
    
    if [[ $# -eq 0 ]]; then
        # Clone all repos
        repos_to_clone=("${!REPOS[@]}")
    else
        # Clone specified repos
        for arg in "$@"; do
            if [[ -v "REPOS[$arg]" ]]; then
                repos_to_clone+=("$arg")
            else
                log_warn "Unknown repository: $arg"
                echo "Available repositories:"
                for repo in "${!REPOS[@]}"; do
                    echo "  - $repo"
                done
                exit 1
            fi
        done
    fi

    # Clone/update repositories
    for name in "${repos_to_clone[@]}"; do
        clone_or_update "$name" "${REPOS[$name]}"
    done

    echo ""
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║                        Setup Complete                         ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo ""
    echo "Upstream repositories are in: $UPSTREAM_DIR"
    echo ""
    echo "Key files to study:"
    echo "  cosmic-comp:"
    echo "    - src/input/mod.rs (input handling)"
    echo "    - src/wayland/protocols/screencopy.rs (capture example)"
    echo ""
    echo "  xdg-desktop-portal-cosmic:"
    echo "    - src/screencast.rs (portal implementation pattern)"
    echo "    - src/main.rs (registration)"
    echo ""
    echo "  rustdesk:"
    echo "    - libs/scrap/src/wayland/remote_desktop_portal.rs (client)"
    echo ""
}

# Show help
if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    echo "Usage: $0 [repo1] [repo2] ..."
    echo ""
    echo "Clone or update upstream reference repositories."
    echo ""
    echo "Available repositories:"
    for repo in "${!REPOS[@]}"; do
        echo "  - $repo"
    done
    echo ""
    echo "If no repositories are specified, all will be cloned."
    exit 0
fi

main "$@"

