# Build the CLI binary.
#
# Wraps `cargo build` against the workspace's root crate (the
# `game-of-life` console binary). Honours the standard PROFILE env
# var convention used by other scripts in this folder.
#
# Usage:
#   .\scripts\build-cli.ps1                          # release build (default)
#   $env:PROFILE = 'debug'; .\scripts\build-cli.ps1  # debug build
#   .\scripts\build-cli.ps1 -- --features foo        # forward extra args to cargo

[CmdletBinding()]
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$CargoArgs
)

$ErrorActionPreference = 'Stop'

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
Push-Location $RepoRoot
try {
    # Avoid the automatic $profile variable.
    $buildProfile = if ($env:PROFILE) { $env:PROFILE } else { 'release' }

    switch ($buildProfile) {
        'release' {
            Write-Host '--> [cli] cargo build --release'
            & cargo build --release @CargoArgs
            $binDir = 'target\release'
        }
        'debug' {
            Write-Host '--> [cli] cargo build'
            & cargo build @CargoArgs
            $binDir = 'target\debug'
        }
        default {
            Write-Error "Unknown PROFILE='$buildProfile' (expected 'release' or 'debug')"
        }
    }

    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $exe = if ($IsWindows -or $env:OS -eq 'Windows_NT') { '.exe' } else { '' }
    Write-Host "--> [cli] binary: $binDir\game-of-life$exe"
}
finally {
    Pop-Location
}
