<#
.SYNOPSIS
    End-to-end smoke test for every bundled example board (Windows).

.DESCRIPTION
    PowerShell counterpart to scripts/examples-smoke-test.sh. Loads every
    file in examples/patterns/*.gol through the game-of-life binary with
    --load-board, runs a sensible number of iterations, and asserts that
    the binary exits cleanly and emits the expected lifecycle markers.

    CI uses the bash version on both Windows and Linux runners (via Git
    Bash on Windows). This script exists for local Windows development
    where bash may not be on PATH.

.PARAMETER Profile
    Build profile to test. "release" (default) or "debug".

.EXAMPLE
    .\scripts\examples-smoke-test.ps1

.EXAMPLE
    .\scripts\examples-smoke-test.ps1 -Profile debug
#>
[CmdletBinding()]
param(
    [ValidateSet('release', 'debug')]
    [string]$Profile = 'release'
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

$bin = Join-Path $repoRoot "target\$Profile\game-of-life.exe"
if (-not (Test-Path $bin)) {
    Write-Host "Binary not found at $bin; building" -ForegroundColor Yellow
    if ($Profile -eq 'release') {
        & cargo build --release
    } else {
        & cargo build
    }
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit $LASTEXITCODE)" }
}

# Methuselahs and the gun get more iterations so they actually do something
# interesting; the 1000x1000 random board gets only a handful so the local
# loop stays fast.
function Get-Iterations {
    param([string]$Name)
    switch ($Name) {
        'r-pentomino-methuselah.gol' { return 50 }
        'acorn-methuselah.gol'       { return 50 }
        'gosper-glider-gun.gol'      { return 50 }
        'random-1000x1000.gol'       { return 5  }
        default                       { return 20 }
    }
}

$boards = Get-ChildItem -Path (Join-Path $repoRoot 'examples\patterns') -Filter '*.gol' | Sort-Object Name
if ($boards.Count -eq 0) {
    Write-Error 'No example boards found under examples/patterns/'
    exit 1
}

# Private runs-dir so the smoke test never pollutes the working tree.
$runsDir = Join-Path ([IO.Path]::GetTempPath()) ("gol-examples-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $runsDir -Force | Out-Null

$requiredMarkers = @(
    '^Initial board: load:',
    '^Generation 0:',
    '^Simulation complete:',
    '^Saved run record:'
)

$failures = 0
try {
    foreach ($board in $boards) {
        $iters = Get-Iterations -Name $board.Name
        $log = Join-Path $runsDir ($board.Name + '.log')
        Write-Host "--> [examples] $($board.Name) (iters=$iters)"

        & $bin --load-board $board.FullName --max-iterations $iters --runs-dir $runsDir *> $log
        $exit = $LASTEXITCODE
        if ($exit -ne 0) {
            Write-Host "FAIL: $($board.Name) exited $exit" -ForegroundColor Red
            Get-Content $log | ForEach-Object { "    $_" } | Write-Host
            $failures++
            continue
        }

        $content = Get-Content $log
        $missing = $requiredMarkers | Where-Object { -not ($content -match $_) }
        if ($missing) {
            Write-Host "FAIL: $($board.Name) missing markers: $($missing -join ', ')" -ForegroundColor Red
            $content | ForEach-Object { "    $_" } | Write-Host
            $failures++
        }
    }
} finally {
    Remove-Item -Recurse -Force $runsDir -ErrorAction SilentlyContinue
}

if ($failures -gt 0) {
    Write-Host "[examples] FAILED: $failures of $($boards.Count) boards" -ForegroundColor Red
    exit 1
}

Write-Host "[examples] PASSED: all $($boards.Count) boards loaded and ran" -ForegroundColor Green
