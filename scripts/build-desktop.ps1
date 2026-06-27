# Produce a release build of the desktop visualizer.
#
# Mirrors the README workflow: runs `cargo tauri build` from
# desktop\, which builds the React frontend (vite build) and then
# compiles + bundles the Rust shell. Output bundles land under
# desktop\target\release\bundle\.
#
# Usage:
#   .\scripts\build-desktop.ps1                # release bundle (default)
#   .\scripts\build-desktop.ps1 -- --debug     # forward args to `cargo tauri build`
#
# Re-runs `npm install` only when node_modules is missing. Set
# $env:FORCE_INSTALL = '1' to reinstall every time. Set
# $env:SKIP_TAURI_INSTALL = '1' to skip the `cargo install tauri-cli`
# probe.

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
    Write-Host '--> [desktop] running cargo tauri build'
    & cargo tauri build @TauriArgs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Host '--> [desktop] bundles written to desktop\target\release\bundle\'
} finally {
    Pop-Location
}
