#!/usr/bin/env bash
# One-shot: prepare x64-only macOS resources (Node + uv + Claude Code).
# Use when building a separate Intel DMG on an Apple Silicon host.
# Re-run scripts/prepare-resources.sh to restore the universal/native bundle.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RES="$SCRIPT_DIR/../src-tauri/resources"
NODE_VERSION="v22.14.0"

echo "=== Cleaning resources ==="
rm -rf "$RES/node" "$RES/uv" "$RES/claude-code"
mkdir -p "$RES/node" "$RES/uv/bin" "$RES/claude-code"

echo "=== Node.js x64 ==="
curl -fSL "https://nodejs.org/dist/${NODE_VERSION}/node-${NODE_VERSION}-darwin-x64.tar.gz" \
  | tar xz -C "$RES/node" --strip-components=1
file "$RES/node/bin/node"

echo "=== uv x64 ==="
rm -rf /tmp/uv-x64
mkdir -p /tmp/uv-x64
curl -fSL "https://github.com/astral-sh/uv/releases/latest/download/uv-x86_64-apple-darwin.tar.gz" \
  | tar xz -C /tmp/uv-x64
UV_BIN=$(find /tmp/uv-x64 -name uv -type f | head -1)
cp "$UV_BIN" "$RES/uv/bin/uv"
chmod +x "$RES/uv/bin/uv"
rm -rf /tmp/uv-x64
file "$RES/uv/bin/uv"

echo "=== Claude Code (x64 native binary) ==="
cd "$RES/claude-code"
npm init -y > /dev/null
# --ignore-scripts: postinstall (install.cjs) detects host arch via process.arch
# and would copy the arm64 binary on this M1 host. We'll place the x64 binary
# manually below.
npm install --ignore-scripts --no-audit --no-fund --save --force \
  --cpu=x64 --os=darwin \
  @anthropic-ai/claude-code \
  @anthropic-ai/claude-code-darwin-x64

SRC="$RES/claude-code/node_modules/@anthropic-ai/claude-code-darwin-x64/claude"
DEST="$RES/claude-code/node_modules/@anthropic-ai/claude-code/bin/claude.exe"
if [ ! -f "$SRC" ]; then
  echo "ERROR: x64 platform binary missing at $SRC" >&2
  exit 1
fi
mkdir -p "$(dirname "$DEST")"
cp -f "$SRC" "$DEST"
chmod +x "$DEST"
file "$DEST"

echo "=== Resources ready (x64-only) ==="
