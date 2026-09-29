<#
.SYNOPSIS
Builds a Windows debug executable in Docker Linux using the installed SDK/MSVC.
.DESCRIPTION
Requires Docker Desktop with Linux containers and Visual Studio Build Tools.
The source, SDK and MSVC mounts are read-only. Caches stay in Docker volumes;
the resulting executable is copied to target/cross-windows/xemnas.exe.
Compilation does not grant permission to execute the binary under Windows policy.
.EXAMPLE
pwsh -File tools/build-windows-cross.ps1
#>
[CmdletBinding()]
param(
    [string]$SdkRoot = "${env:ProgramFiles(x86)}\Windows Kits\10",
    [string]$MsvcRoot = ''
)
$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$SdkRoot = (Resolve-Path -LiteralPath $SdkRoot).Path
$sdkVersion = Get-ChildItem -LiteralPath (Join-Path $SdkRoot 'Include') -Directory |
    Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' -and
        (Test-Path -LiteralPath (Join-Path $SdkRoot "Lib/$($_.Name)/um/x64/kernel32.lib")) } |
    Sort-Object { [version]$_.Name } -Descending | Select-Object -First 1 -ExpandProperty Name
if (-not $sdkVersion) { throw 'No complete x64 Windows SDK found.' }
if (-not $MsvcRoot) {
    $msvcCandidates = @(
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC",
        "$env:ProgramFiles\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC"
    )
    $MsvcRoot = $msvcCandidates | Where-Object { Test-Path -LiteralPath $_ } |
        ForEach-Object { Get-ChildItem -LiteralPath $_ -Directory } |
        Where-Object { $_.Name -match '^\d+\.\d+\.\d+$' } |
        Sort-Object { [version]$_.Name } -Descending | Select-Object -First 1 -ExpandProperty FullName
}
if (-not $MsvcRoot) { throw 'MSVC not found. Pass -MsvcRoot with the installed VC/Tools/MSVC/version directory.' }
$MsvcRoot = (Resolve-Path -LiteralPath $MsvcRoot).Path
$imageName = 'xemnas-windows-builder:local'
& docker build --file (Join-Path $PSScriptRoot 'windows-cross.Dockerfile') --tag $imageName $PSScriptRoot
if ($LASTEXITCODE -ne 0) { throw 'Could not prepare the cross-compilation image.' }
$containerName = "xemnas-cross-$([guid]::NewGuid().ToString('N'))"
$outputDir = Join-Path $repoRoot 'target/cross-windows'
New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
try {
    & docker run --name $containerName `
        --mount "type=bind,source=$repoRoot,target=/workspace,readonly" `
        --mount "type=bind,source=$SdkRoot,target=/win-sdk,readonly" `
        --mount "type=bind,source=$MsvcRoot,target=/msvc,readonly" `
        --mount 'type=volume,source=xemnas-linux-target,target=/build' `
        --mount 'type=volume,source=xemnas-linux-cargo,target=/usr/local/cargo' `
        --env "XEMNAS_SDK_VERSION=$sdkVersion" $imageName sh /workspace/tools/windows-cross.sh
    if ($LASTEXITCODE -ne 0) { throw 'Windows cross-compilation failed; no executable was exported.' }
    $outputExe = Join-Path $outputDir 'xemnas.exe'
    & docker cp "${containerName}:/build/x86_64-pc-windows-msvc/debug/xemnas.exe" $outputExe
    if ($LASTEXITCODE -ne 0) { throw 'Could not export the compiled executable.' }
    Write-Output "Compiled: $outputExe"
    Write-Output "SHA256: $((Get-FileHash -LiteralPath $outputExe -Algorithm SHA256).Hash)"
} finally {
    & docker rm $containerName | Out-Null
}
