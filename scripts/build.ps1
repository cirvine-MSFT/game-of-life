# Build everything: CLI binary first, then the desktop bundle.
#
# Delegates to the per-component scripts so this stays a thin
# orchestrator. The CLI builds first because:
#   1. It's the fastest path to failure if the workspace is broken.
#   2. `cargo tauri build` shares the same Cargo target dir, so any
#      shared dependencies are already compiled by the time the
#      desktop step starts.
#
# Usage:
#   .\scripts\build.ps1                      # release everything (default)
#   $env:PROFILE = 'debug'; .\scripts\build.ps1  # debug CLI; desktop is always release
#
# Environment variables forwarded to child scripts:
#   PROFILE              -> build-cli.ps1 (release|debug)
#   FORCE_INSTALL        -> build-desktop.ps1 (re-run npm install)
#   SKIP_TAURI_INSTALL   -> build-desktop.ps1 (skip cargo install tauri-cli probe)

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$ScriptDir = $PSScriptRoot

Write-Host '=== [build] step 1/2: CLI ==='
& (Join-Path $ScriptDir 'build-cli.ps1')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host ''
Write-Host '=== [build] step 2/2: desktop ==='
& (Join-Path $ScriptDir 'build-desktop.ps1')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host ''
Write-Host '=== [build] done ==='
