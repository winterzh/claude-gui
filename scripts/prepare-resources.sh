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

# --- uv (Python package runner for MCP servers) ---
echo "--- Downloading uv (arm64) ---"
mkdir -p /tmp/uv-arm64 /tmp/uv-x64 "$RESOURCE_DIR/uv/bin"
curl -fSL "https://github.com/astral-sh/uv/releases/latest/download/uv-aarch64-apple-darwin.tar.gz" | tar xz -C /tmp/uv-arm64

echo "--- Downloading uv (x64) ---"
curl -fSL "https://github.com/astral-sh/uv/releases/latest/download/uv-x86_64-apple-darwin.tar.gz" | tar xz -C /tmp/uv-x64

echo "--- Creating universal uv binary ---"
# uv tarballs extract to uv-*/uv
UV_ARM64=$(find /tmp/uv-arm64 -name uv -type f | head -1)
UV_X64=$(find /tmp/uv-x64 -name uv -type f | head -1)
lipo -create "$UV_ARM64" "$UV_X64" -output "$RESOURCE_DIR/uv/bin/uv"
chmod +x "$RESOURCE_DIR/uv/bin/uv"
rm -rf /tmp/uv-arm64 /tmp/uv-x64
echo "--- Verifying universal uv binary ---"
file "$RESOURCE_DIR/uv/bin/uv"

echo "--- Installing @anthropic-ai/claude-code ---"
# Since 2.1.x, the package ships a native binary; postinstall (install.cjs)
# copies the platform binary into bin/claude.exe. Don't silence stderr — if
# postinstall fails, we want to see why.
cd "$RESOURCE_DIR/claude-code"
"$RESOURCE_DIR/node/bin/node" "$RESOURCE_DIR/node/lib/node_modules/npm/bin/npm-cli.js" init -y > /dev/null
"$RESOURCE_DIR/node/bin/node" "$RESOURCE_DIR/node/lib/node_modules/npm/bin/npm-cli.js" install @anthropic-ai/claude-code@latest --save --no-audit --no-fund

CLAUDE_BIN="$RESOURCE_DIR/claude-code/node_modules/@anthropic-ai/claude-code/bin/claude.exe"
if [ ! -f "$CLAUDE_BIN" ]; then
  echo "ERROR: Claude Code native binary missing at $CLAUDE_BIN" >&2
  echo "Postinstall (install.cjs) likely failed. Check npm output above." >&2
  exit 1
fi
file "$CLAUDE_BIN"

echo "=== Resources ready ==="
