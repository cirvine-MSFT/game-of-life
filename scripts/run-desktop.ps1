# Build and launch the desktop visualizer in development mode.
#
# Wraps the documented workflow from README.md: runs `cargo tauri dev
# --no-watch` from desktop\, after making sure tauri-cli is installed
# and frontend dependencies are present in desktop\ui\. `--no-watch`
# is deliberate — Tauri's Rust-source watcher currently picks up
# changes inside desktop\ui\node_modules\ and triggers spurious
# rebuilds; the Vite dev server still hot-reloads the frontend.
#
# Usage:
#   .\scripts\run-desktop.ps1                  # dev mode (default)
#   .\scripts\run-desktop.ps1 -- --release     # forward args to `cargo tauri dev`
#
# Re-runs `npm install` only when node_modules is missing. Set
# $env:FORCE_INSTALL = '1' to reinstall every time. Set
# $env:SKIP_TAURI_INSTALL = '1' to skip the `cargo install tauri-cli`
# probe (useful in CI).

[CmdletBinding()]
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$TauriArgs
)

$ErrorActionPreference = 'Stop'

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$DesktopDir = Join-Path $RepoRoot 'desktop'
$UiDir = Join-Path $DesktopDir 'ui'

if (-not (Test-Path $DesktopDir) -or -not (Test-Path $UiDir)) {
    Write-Error "Expected desktop\ and desktop\ui\ under $RepoRoot"
}

if ($env:SKIP_TAURI_INSTALL -ne '1') {
    & cargo tauri --version *> $null
    if ($LASTEXITCODE -ne 0) {
        Write-Host '--> [desktop] installing tauri-cli (one-time, ~couple of minutes)'
        & cargo install tauri-cli --version "^2.0" --locked
        if ($LASTEXITCODE -ne 0) { throw "cargo install tauri-cli failed ($LASTEXITCODE)" }
    }
}

if ($env:FORCE_INSTALL -eq '1' -or -not (Test-Path (Join-Path $UiDir 'node_modules'))) {
    Write-Host '--> [desktop] installing UI dependencies'
    Push-Location $UiDir
    try {
        npm install
        if ($LASTEXITCODE -ne 0) { throw "npm install failed ($LASTEXITCODE)" }
    } finally {
        Pop-Location
    }
}

Push-Location $DesktopDir
try {
    Write-Host '--> [desktop] launching cargo tauri dev --no-watch'
    & cargo tauri dev --no-watch @TauriArgs
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
