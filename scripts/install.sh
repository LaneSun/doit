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
cd "$PROJECT_DIR"

# ---- Build frontend (web UI) ----
# rust-embed 在 release 构建时把 web/build 嵌入二进制,故需先产出前端资源。
if command -v bun &>/dev/null; then
    info "Building web frontend (bun)..."
    (cd web && bun install --frozen-lockfile 2>/dev/null || bun install)
    (cd web && bun run build)
elif [[ ! -d web/build || -z "$(ls -A web/build 2>/dev/null)" ]]; then
    warn "bun not found and web/build is empty; the web UI will not be available."
    mkdir -p web/build
    echo '<!doctype html><meta charset="utf-8"><title>doit</title><p>Frontend not built.' > web/build/index.html
else
    warn "bun not found; reusing existing web/build."
fi

# ---- Build ----
info "Building doit (release mode)..."
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
