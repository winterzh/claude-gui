#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RESOURCE_DIR="$SCRIPT_DIR/../src-tauri/resources"
NODE_VERSION="v22.14.0"

echo "=== Preparing resources ==="
mkdir -p "$RESOURCE_DIR/node" "$RESOURCE_DIR/claude-code"

OS="$(uname -s)"
ARCH="$(uname -m)"
case "$ARCH" in x86_64|amd64) NODE_ARCH="x64";; aarch64|arm64) NODE_ARCH="arm64";; *) echo "Unsupported: $ARCH"; exit 1;; esac

if [ "$OS" = "Darwin" ]; then PLATFORM="darwin"; else PLATFORM="linux"; fi

NODE_URL="https://nodejs.org/dist/${NODE_VERSION}/node-${NODE_VERSION}-${PLATFORM}-${NODE_ARCH}.tar.gz"
echo "--- Downloading Node.js $NODE_VERSION ($PLATFORM-$NODE_ARCH) ---"
curl -fSL "$NODE_URL" | tar xz -C "$RESOURCE_DIR/node" --strip-components=1
echo "--- Installing @anthropic-ai/claude-code ---"
cd "$RESOURCE_DIR/claude-code"
"$RESOURCE_DIR/node/bin/node" "$RESOURCE_DIR/node/bin/npm" init -y > /dev/null 2>&1
"$RESOURCE_DIR/node/bin/node" "$RESOURCE_DIR/node/bin/npm" install @anthropic-ai/claude-code --save > /dev/null 2>&1
echo "=== Resources ready ==="
