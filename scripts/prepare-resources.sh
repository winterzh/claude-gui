#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RESOURCE_DIR="$SCRIPT_DIR/../src-tauri/resources"
NODE_VERSION="v22.14.0"

echo "=== Preparing resources ==="
mkdir -p "$RESOURCE_DIR/node-arm64" "$RESOURCE_DIR/node-x64" "$RESOURCE_DIR/node/bin" "$RESOURCE_DIR/claude-code"

OS="$(uname -s)"
if [ "$OS" = "Darwin" ]; then PLATFORM="darwin"; else PLATFORM="linux"; fi

# Download both architectures for macOS universal build
echo "--- Downloading Node.js $NODE_VERSION (arm64) ---"
curl -fSL "https://nodejs.org/dist/${NODE_VERSION}/node-${NODE_VERSION}-${PLATFORM}-arm64.tar.gz" | tar xz -C "$RESOURCE_DIR/node-arm64" --strip-components=1

echo "--- Downloading Node.js $NODE_VERSION (x64) ---"
curl -fSL "https://nodejs.org/dist/${NODE_VERSION}/node-${NODE_VERSION}-${PLATFORM}-x64.tar.gz" | tar xz -C "$RESOURCE_DIR/node-x64" --strip-components=1

# Create universal node binary with lipo
echo "--- Creating universal node binary ---"
lipo -create "$RESOURCE_DIR/node-arm64/bin/node" "$RESOURCE_DIR/node-x64/bin/node" -output "$RESOURCE_DIR/node/bin/node"
chmod +x "$RESOURCE_DIR/node/bin/node"

# Copy npm/npx from arm64 (JS files are arch-independent)
cp -R "$RESOURCE_DIR/node-arm64/lib" "$RESOURCE_DIR/node/lib"
cp "$RESOURCE_DIR/node-arm64/bin/npm" "$RESOURCE_DIR/node/bin/npm" 2>/dev/null || true
cp "$RESOURCE_DIR/node-arm64/bin/npx" "$RESOURCE_DIR/node/bin/npx" 2>/dev/null || true

# Cleanup single-arch downloads
rm -rf "$RESOURCE_DIR/node-arm64" "$RESOURCE_DIR/node-x64"

# Verify
echo "--- Verifying universal binary ---"
file "$RESOURCE_DIR/node/bin/node"

echo "--- Installing @anthropic-ai/claude-code ---"
cd "$RESOURCE_DIR/claude-code"
"$RESOURCE_DIR/node/bin/node" "$RESOURCE_DIR/node/lib/node_modules/npm/bin/npm-cli.js" init -y > /dev/null 2>&1
"$RESOURCE_DIR/node/bin/node" "$RESOURCE_DIR/node/lib/node_modules/npm/bin/npm-cli.js" install @anthropic-ai/claude-code --save > /dev/null 2>&1
echo "=== Resources ready ==="
