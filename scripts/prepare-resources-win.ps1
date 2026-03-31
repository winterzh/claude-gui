# Download portable Node.js + install Claude Code for bundling
$ErrorActionPreference = "Stop"

$ResourceDir = Join-Path $PSScriptRoot "..\src-tauri\resources"
$NodeVersion = "v22.14.0"

Write-Host "=== Preparing resources ==="

# Create dirs
New-Item -ItemType Directory -Force -Path "$ResourceDir\node" | Out-Null
New-Item -ItemType Directory -Force -Path "$ResourceDir\claude-code" | Out-Null

# Download Node.js portable for Windows x64
$NodeUrl = "https://nodejs.org/dist/$NodeVersion/node-$NodeVersion-win-x64.zip"
$Archive = "$ResourceDir\node-archive.zip"

Write-Host "--- Downloading Node.js $NodeVersion ---"
Invoke-WebRequest -Uri $NodeUrl -OutFile $Archive

Write-Host "--- Extracting ---"
Expand-Archive -Path $Archive -DestinationPath "$ResourceDir\node-tmp" -Force

# Copy node.exe + npm/npx
$NodeDir = "$ResourceDir\node-tmp\node-$NodeVersion-win-x64"
Copy-Item "$NodeDir\node.exe" "$ResourceDir\node\node.exe"
Copy-Item -Recurse "$NodeDir\node_modules" "$ResourceDir\node\node_modules"
Copy-Item "$NodeDir\npx.cmd" "$ResourceDir\node\npx.cmd"
Copy-Item "$NodeDir\npm.cmd" "$ResourceDir\node\npm.cmd"

Remove-Item -Recurse -Force "$ResourceDir\node-tmp", $Archive

Write-Host "--- Installing @anthropic-ai/claude-code ---"
Push-Location "$ResourceDir\claude-code"

# Use the bundled node/npm to install
$env:PATH = "$ResourceDir\node;$env:PATH"
& "$ResourceDir\node\npm.cmd" init -y 2>$null | Out-Null
& "$ResourceDir\node\npm.cmd" install @anthropic-ai/claude-code --save 2>$null | Out-Null

Pop-Location

Write-Host "=== Resources ready ==="
Write-Host "Node: $(Get-ChildItem $ResourceDir\node\node.exe)"
Write-Host "Claude Code: $(Get-ChildItem $ResourceDir\claude-code\node_modules\@anthropic-ai\claude-code\cli.js)"
