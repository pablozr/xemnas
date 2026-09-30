<#
.SYNOPSIS
    Compiles the xemnas Windows resources (icon + version info) into xemnas.res.

.DESCRIPTION
    The mark's source of truth is apps\desktop-gpui\assets\brand\xemnas-mark.svg.
    Render it to PNGs (16, 20, 24, 32, 40, 48, 64, 256 px, each natively) and
    pack them into xemnas.ico, then run this script. The resulting xemnas.res is
    versioned and linked into every Windows MSVC binary by .cargo\config.toml,
    so builds never run a freshly compiled build script (this machine's
    Application Control policy blocks new unsigned executables).

.EXAMPLE
    powershell -File tools\brand-icon.ps1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$brand = Join-Path $PSScriptRoot '..\apps\desktop-gpui\assets\brand' | Resolve-Path
$kits = 'C:\Program Files (x86)\Windows Kits\10'
$sdk = Get-ChildItem -Path (Join-Path $kits 'bin') -Directory |
    Where-Object { Test-Path (Join-Path $_.FullName 'x64\rc.exe') } |
    Sort-Object Name -Descending |
    Select-Object -First 1
if (-not $sdk) { throw 'rc.exe not found; install the Windows SDK.' }
$rcExe = Join-Path $sdk.FullName 'x64\rc.exe'
$include = Join-Path $kits "Include\$($sdk.Name)"

Push-Location $brand
try {
    & $rcExe /nologo /i "$include\um" /i "$include\shared" /fo xemnas.res xemnas.rc
    if ($LASTEXITCODE -ne 0) { throw "rc.exe failed with $LASTEXITCODE" }
    Write-Host "BRAND $brand\xemnas.res"
} finally {
    Pop-Location
}
