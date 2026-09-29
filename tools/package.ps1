<#
.SYNOPSIS
    Builds the release binary and produces the Windows distribution ZIP.

.DESCRIPTION
    One command, one profile, one source of truth for the version
    (`[workspace.package]` in the root `Cargo.toml`) and `--locked` for the
    dependency set. The archive is a plain ZIP named deterministically:

        dist/xemnas-windows-x86_64-<version>.zip

    containing `xemnas.exe` plus `LICENSE`/`README.md` when those files exist at
    the repo root (nothing is invented).

    The packaged profile is a fixed constant (`release`), not a parameter: the
    build command and the staged path are derived from the same value, so the
    script can never announce one profile while shipping another. The staged file
    is hash-compared with the built binary before archiving; a missing binary
    fails loudly instead of shipping a stale artifact.

    Reproducibility scope: the lockfile, the version and the build command are
    fixed, and CI repeats the same path. Bit-identical archives are NOT promised
    — ZIP entry timestamps and Windows build metadata vary between runs. That
    limitation is deliberate and recorded with the ADR.

    The script fails loudly (`$ErrorActionPreference='Stop'`, non-zero exit) and
    is idempotent: an old staging directory and a previous ZIP are removed first.

.EXAMPLE
    powershell -NoProfile -File tools\package.ps1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

# cargo lives under the user profile in this environment.
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path

# Distribution profile. Fixed on purpose so build and staging can never drift:
# `$profileName` is the `target` subdirectory and `$profileArgs` is the cargo
# flag. Tests may copy this script and flip both values together to exercise the
# pipeline on another profile; production usage never changes them.
$profileName = 'release'
$profileArgs = @('--release')

# --- version ---------------------------------------------------------------

$rootCargo = Get-Content -LiteralPath (Join-Path $repoRoot 'Cargo.toml') -Raw
$versionMatch = [regex]::Match($rootCargo, '(?m)^\s*version\s*=\s*"([^"]+)"')
if (-not $versionMatch.Success) {
    throw 'workspace version not found in Cargo.toml'
}
$version = $versionMatch.Groups[1].Value

# --- build -----------------------------------------------------------------

Write-Host "PACKAGE building xemnas ($profileName, locked)"
Push-Location $repoRoot
try {
    & cargo build @profileArgs --locked -p desktop-gpui --bin xemnas
    $code = $LASTEXITCODE
} finally {
    Pop-Location
}
if ($code -ne 0) {
    throw "cargo build for the $profileName profile failed with exit $code"
}

$exe = Join-Path $repoRoot "target\$profileName\xemnas.exe"
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
    throw "the $profileName binary was not produced at $exe; refusing to package a stale artifact"
}

# --- staging ---------------------------------------------------------------

$distDir = Join-Path $repoRoot 'dist'
$staging = Join-Path $distDir 'staging'
Remove-Item -LiteralPath $staging -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $staging | Out-Null

$stagedExe = Join-Path $staging 'xemnas.exe'
Copy-Item -LiteralPath $exe -Destination $stagedExe -Force

# Prove the archive will carry exactly the binary this run points at.
$builtHash = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash
$stagedHash = (Get-FileHash -LiteralPath $stagedExe -Algorithm SHA256).Hash
if ($builtHash -ne $stagedHash) {
    throw 'staging does not match the built binary; refusing to archive'
}

foreach ($name in @('LICENSE', 'README.md')) {
    $candidate = Join-Path $repoRoot $name
    if (Test-Path -LiteralPath $candidate -PathType Leaf) {
        Copy-Item -LiteralPath $candidate -Destination (Join-Path $staging $name) -Force
    }
}

# --- archive ---------------------------------------------------------------

$zipName = "xemnas-windows-x86_64-$version.zip"
$zipPath = Join-Path $distDir $zipName
Remove-Item -LiteralPath $zipPath -Force -ErrorAction SilentlyContinue
Compress-Archive -Path (Join-Path $staging '*') -DestinationPath $zipPath -CompressionLevel Optimal
Remove-Item -LiteralPath $staging -Recurse -Force -ErrorAction SilentlyContinue

if (-not (Test-Path -LiteralPath $zipPath)) {
    throw "archive was not created at $zipPath"
}

Write-Output $zipPath
