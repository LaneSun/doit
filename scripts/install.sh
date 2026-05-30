#!/usr/bin/env bash
set -euo pipefail

# ----------------------------------------
# doit install script
# Usage: ./scripts/install.sh [--prefix <path>]
#
# Default install prefix: $HOME/.local
# Binary installed to:   <prefix>/bin/doit
# ----------------------------------------

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

die() {
    echo -e "${RED}error:${NC} $*" >&2
    exit 1
}

info() {
    echo -e "${GREEN}=>${NC} $*"
}

warn() {
    echo -e "${YELLOW}warning:${NC} $*" >&2
}

# ---- Parse arguments ----
PREFIX="${HOME}/.local"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --prefix)
            PREFIX="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: $0 [--prefix <path>]"
            echo "Install doit to the specified prefix (default: \$HOME/.local)"
            exit 0
            ;;
        *)
            die "Unknown option: $1"
            ;;
    esac
done

BINDIR="${PREFIX}/bin"
BINPATH="${BINDIR}/doit"

# ---- Prerequisites ----
command -v cargo &>/dev/null || die "cargo not found. Install Rust from https://rustup.rs"

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

# ---- Build ----
info "Building doit (release mode)..."
cd "$PROJECT_DIR"
cargo build --release

# ---- Install ----
mkdir -p "$BINDIR"
cp -f target/release/doit "$BINPATH"
chmod 755 "$BINPATH"

info "doit installed to: $BINPATH"

# ---- PATH check ----
if [[ ":${PATH}:" != *":${BINDIR}:"* ]]; then
    warn "${BINDIR} is not in your PATH."
    echo "  Add the following to your shell rc file (~/.bashrc, ~/.zshrc, etc.):"
    echo "    export PATH=\"${BINDIR}:\$PATH\""
fi
