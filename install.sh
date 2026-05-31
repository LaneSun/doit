#!/usr/bin/env sh

set -e

REPO="LaneSun/doit"
INSTALL_DIR=""
FORCE=0

# Parse arguments
for arg in "$@"; do
    case "$arg" in
        --force|-f) FORCE=1 ;;
        --help|-h)
            echo "Usage: curl -fsSL https://raw.githubusercontent.com/LaneSun/doit/main/install.sh | sh"
            echo ""
            echo "Options:"
            echo "  --force, -f    Force reinstall even if already at latest version"
            echo "  --help, -h     Show this help message"
            exit 0
            ;;
    esac
done

# Detect OS
OS=$(uname -s)
case "$OS" in
    Linux*)     OS_TYPE="linux";;
    Darwin*)    OS_TYPE="macos";;
    *)          echo "Error: Unsupported OS: $OS"; exit 1;;
esac

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
    x86_64)     ARCH_TYPE="x86_64";;
    amd64)      ARCH_TYPE="x86_64";;
    arm64)      ARCH_TYPE="aarch64";;
    aarch64)    ARCH_TYPE="aarch64";;
    *)          echo "Error: Unsupported architecture: $ARCH"; exit 1;;
esac

# Build target string
if [ "$OS_TYPE" = "linux" ]; then
    TARGET="${ARCH_TYPE}-unknown-linux-gnu"
else
    TARGET="${ARCH_TYPE}-apple-darwin"
fi

# Determine install directory
if [ -w /usr/local/bin ] 2>/dev/null || [ "$(id -u)" -eq 0 ]; then
    INSTALL_DIR="/usr/local/bin"
else
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
fi

# Function to get latest version from GitHub API
get_latest_version() {
    local api_url="https://api.github.com/repos/${REPO}/releases/latest"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$api_url" 2>/dev/null | grep -o '"tag_name": *"v[^"]*"' | sed 's/.*"v\([^"]*\)".*/\1/'
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "$api_url" 2>/dev/null | grep -o '"tag_name": *"v[^"]*"' | sed 's/.*"v\([^"]*\)".*/\1/'
    fi
}

# Function to compare versions (returns 0 if v1 < v2)
version_lt() {
    if [ "$1" = "$2" ]; then
        return 1
    fi
    printf '%s\n%s\n' "$1" "$2" | sort -t. -k1,1n -k2,2n -k3,3n | head -n1 | grep -qx "$1"
}

# Check if doit is already installed
SHOULD_INSTALL=1
if [ "$FORCE" -eq 0 ] && command -v doit >/dev/null 2>&1; then
    EXISTING_PATH=$(command -v doit)
    echo "doit is already installed at: $EXISTING_PATH"

    INSTALLED_VERSION=""
    if doit --version >/dev/null 2>&1; then
        INSTALLED_VERSION=$(doit --version | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+' | head -n1)
        echo "Installed version: v${INSTALLED_VERSION}"
    fi

    LATEST_VERSION=$(get_latest_version)
    if [ -n "$LATEST_VERSION" ]; then
        echo "Latest version: v${LATEST_VERSION}"
    fi

    if [ -n "$INSTALLED_VERSION" ] && [ -n "$LATEST_VERSION" ]; then
        if version_lt "$INSTALLED_VERSION" "$LATEST_VERSION"; then
            echo "Newer version available. Auto-updating..."
            SHOULD_INSTALL=1
        elif [ "$INSTALLED_VERSION" = "$LATEST_VERSION" ]; then
            echo "Already at the latest version."
            if [ -t 0 ] || [ -c /dev/tty ]; then
                printf "Reinstall anyway? [y/N] "
                if read -r REPLY < /dev/tty 2>/dev/null; then
                    case "$REPLY" in
                        y|Y|yes|YES) SHOULD_INSTALL=1;;
                        *) SHOULD_INSTALL=0;;
                    esac
                else
                    SHOULD_INSTALL=0
                fi
            else
                echo "Use: curl ... | sh -s -- --force  to force reinstall"
                SHOULD_INSTALL=0
            fi
        fi
    fi
fi

if [ "$SHOULD_INSTALL" -eq 0 ]; then
    exit 0
fi

# Create temp directory
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

# Download URL
DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/doit-${TARGET}.tar.gz"

echo ""
echo "Platform: ${OS_TYPE} / ${ARCH_TYPE}"
echo "Downloading doit from GitHub releases..."
echo "  ${DOWNLOAD_URL}"

# Download
cd "$TMP_DIR"
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$DOWNLOAD_URL" -o doit.tar.gz
elif command -v wget >/dev/null 2>&1; then
    wget -q "$DOWNLOAD_URL" -O doit.tar.gz
else
    echo "Error: Neither curl nor wget is installed."
    exit 1
fi

# Extract
echo "Extracting..."
tar xzf doit.tar.gz

# Check if binary exists
if [ ! -f "doit" ]; then
    echo "Error: Expected binary 'doit' not found in archive."
    echo "Archive contents:"
    tar tzf doit.tar.gz
    exit 1
fi

# Install
if [ "$INSTALL_DIR" = "/usr/local/bin" ] && [ ! -w "$INSTALL_DIR" ]; then
    echo "Installing to ${INSTALL_DIR} (requires sudo)..."
    sudo mv doit "$INSTALL_DIR/"
    sudo chmod +x "$INSTALL_DIR/doit"
else
    echo "Installing to ${INSTALL_DIR}..."
    mv doit "$INSTALL_DIR/"
    chmod +x "$INSTALL_DIR/doit"
fi

# Verify
echo ""
echo "doit installed successfully!"
"${INSTALL_DIR}/doit" --version || true

# PATH check
if ! command -v doit >/dev/null 2>&1; then
    case ":$PATH:" in
        *":$INSTALL_DIR:"*)
            echo ""
            echo "Note: doit is installed but not found in current session."
            echo "Restart your terminal or run: source ~/.bashrc"
            ;;
        *)
            echo ""
            echo "Add ${INSTALL_DIR} to your PATH:"
            echo "  echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.bashrc"
            echo "  source ~/.bashrc"
            ;;
    esac
fi

echo ""
echo "Quick start:"
echo "  1. Navigate to a git/jj repository"
echo "  2. Run:  doit prompt"
echo "  3. Or try the web UI:  doit web"
