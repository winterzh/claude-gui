# Download portable Node.js + MinGit + Claude Code for bundling
$ErrorActionPreference = "Stop"

$ResourceDir = Join-Path $PSScriptRoot "..\src-tauri\resources"
$NodeVersion = "v22.14.0"
$GitVersion = "2.49.0"
$MinGitTag = "v${GitVersion}.windows.1"

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

# --- MinGit ---
# Use MinGit with busybox — includes bash.exe which Claude Code requires
$GitUrl = "https://github.com/git-for-windows/git/releases/download/${MinGitTag}/MinGit-${GitVersion}-busybox-64-bit.zip"
$GitArchive = "$ResourceDir\mingit.zip"

Write-Host "--- Downloading MinGit $GitVersion ---"
Invoke-WebRequest -Uri $GitUrl -OutFile $GitArchive

Write-Host "--- Extracting MinGit ---"
Expand-Archive -Path $GitArchive -DestinationPath "$ResourceDir\git" -Force
Remove-Item -Force $GitArchive

# --- Claude Code ---
Write-Host "--- Installing @anthropic-ai/claude-code ---"
Push-Location "$ResourceDir\claude-code"

$env:PATH = "$ResourceDir\node;$ResourceDir\git\cmd;$env:PATH"
& "$ResourceDir\node\npm.cmd" init -y 2>$null | Out-Null
& "$ResourceDir\node\npm.cmd" install @anthropic-ai/claude-code --save 2>$null | Out-Null

Pop-Location

Write-Host "=== Resources ready ==="
Write-Host "Node: $(Get-ChildItem $ResourceDir\node\node.exe)"
Write-Host "Git: $(Get-ChildItem $ResourceDir\git\cmd\git.exe)"
# Check for bash.exe in various locations
$bashPaths = @("$ResourceDir\git\bin\bash.exe", "$ResourceDir\git\usr\bin\bash.exe", "$ResourceDir\git\mingw64\bin\bash.exe")
foreach ($bp in $bashPaths) {
    if (Test-Path $bp) { Write-Host "Bash: $bp"; break }
}
Write-Host "Claude Code: $(Get-ChildItem $ResourceDir\claude-code\node_modules\@anthropic-ai\claude-code\cli.js)"
