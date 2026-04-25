# Download portable Node.js + PortableGit + Claude Code for bundling
$ErrorActionPreference = "Stop"

$ResourceDir = Join-Path $PSScriptRoot "..\src-tauri\resources"
$NodeVersion = "v22.14.0"
$GitVersion = "2.49.0"

Write-Host "=== Preparing resources ==="

New-Item -ItemType Directory -Force -Path "$ResourceDir\node" | Out-Null
New-Item -ItemType Directory -Force -Path "$ResourceDir\claude-code" | Out-Null
New-Item -ItemType Directory -Force -Path "$ResourceDir\git" | Out-Null

# --- Node.js ---
$NodeUrl = "https://nodejs.org/dist/$NodeVersion/node-$NodeVersion-win-x64.zip"
$Archive = "$ResourceDir\node-archive.zip"

Write-Host "--- Downloading Node.js $NodeVersion ---"
Invoke-WebRequest -Uri $NodeUrl -OutFile $Archive

Write-Host "--- Extracting Node.js ---"
Expand-Archive -Path $Archive -DestinationPath "$ResourceDir\node-tmp" -Force

$NodeDir = "$ResourceDir\node-tmp\node-$NodeVersion-win-x64"
Copy-Item "$NodeDir\node.exe" "$ResourceDir\node\node.exe"
Copy-Item -Recurse "$NodeDir\node_modules" "$ResourceDir\node\node_modules"
Copy-Item "$NodeDir\npx.cmd" "$ResourceDir\node\npx.cmd"
Copy-Item "$NodeDir\npm.cmd" "$ResourceDir\node\npm.cmd"
Remove-Item -Recurse -Force "$ResourceDir\node-tmp", $Archive

# --- PortableGit (includes bash.exe) ---
$GitUrl = "https://github.com/git-for-windows/git/releases/download/v${GitVersion}.windows.1/PortableGit-${GitVersion}-64-bit.7z.exe"
$GitExe = "$ResourceDir\portablegit.exe"

Write-Host "--- Downloading PortableGit $GitVersion ---"
Invoke-WebRequest -Uri $GitUrl -OutFile $GitExe

Write-Host "--- Extracting PortableGit ---"
# PortableGit is a self-extracting 7z archive, use -o to specify output dir, -y to auto-confirm
Start-Process -FilePath $GitExe -ArgumentList "-o`"$ResourceDir\git`"", "-y" -Wait -NoNewWindow
Remove-Item -Force $GitExe

# Verify bash.exe exists
$bashPath = "$ResourceDir\git\bin\bash.exe"
if (Test-Path $bashPath) {
    Write-Host "bash.exe found: $bashPath"
} else {
    Write-Host "WARNING: bash.exe not found at $bashPath"
    # List what we have
    Get-ChildItem -Recurse "$ResourceDir\git\bin" -ErrorAction SilentlyContinue | Select-Object -First 20
}

# --- uv (Python package runner for MCP servers) ---
$UvUrl = "https://github.com/astral-sh/uv/releases/latest/download/uv-x86_64-pc-windows-msvc.zip"
$UvArchive = "$ResourceDir\uv-archive.zip"
New-Item -ItemType Directory -Force -Path "$ResourceDir\uv" | Out-Null

Write-Host "--- Downloading uv ---"
Invoke-WebRequest -Uri $UvUrl -OutFile $UvArchive
Expand-Archive -Path $UvArchive -DestinationPath "$ResourceDir\uv" -Force
Remove-Item -Force $UvArchive

# --- Claude Code ---
# Since 2.1.x, @anthropic-ai/claude-code ships a native binary; the package's
# postinstall (install.cjs) copies the platform binary to bin/claude.exe.
# Do NOT silence stderr — if postinstall fails, we need to see why.
Write-Host "--- Installing @anthropic-ai/claude-code ---"
Push-Location "$ResourceDir\claude-code"

$env:PATH = "$ResourceDir\node;$ResourceDir\git\cmd;$env:PATH"
& "$ResourceDir\node\npm.cmd" init -y | Out-Null
& "$ResourceDir\node\npm.cmd" install @anthropic-ai/claude-code@latest --save --no-audit --no-fund
if ($LASTEXITCODE -ne 0) {
    throw "npm install failed (exit $LASTEXITCODE)"
}

Pop-Location

$ClaudeBin = "$ResourceDir\claude-code\node_modules\@anthropic-ai\claude-code\bin\claude.exe"
if (-not (Test-Path $ClaudeBin)) {
    throw "Claude Code native binary missing at $ClaudeBin. Postinstall (install.cjs) likely failed — check npm output above."
}

Write-Host "=== Resources ready ==="
Write-Host "Node: $(Get-ChildItem $ResourceDir\node\node.exe)"
Write-Host "Git: $(Get-ChildItem $ResourceDir\git\cmd\git.exe)"
Write-Host "Bash: $(Get-ChildItem $ResourceDir\git\bin\bash.exe -ErrorAction SilentlyContinue)"
Write-Host "uv: $(Get-ChildItem $ResourceDir\uv\uv.exe -ErrorAction SilentlyContinue)"
Write-Host "Claude Code: $(Get-ChildItem $ClaudeBin)"
