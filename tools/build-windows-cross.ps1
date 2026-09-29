<#
.SYNOPSIS
Builds a Windows release executable in Docker Linux using the installed SDK/MSVC.
.DESCRIPTION
Requires Docker Desktop with Linux containers and Visual Studio Build Tools.
The source, SDK and MSVC mounts are read-only. Caches stay in Docker volumes;
the resulting executable is copied to target/cross-windows/xemnas.exe.
Release is required: GPUI debug builds resolve shader sources from
CARGO_MANIFEST_DIR at runtime, which does not exist on Windows when the binary
was cross-compiled (panic "os error 3"). Release builds embed shader bytes.
The gpui_windows build script only runs compile_shaders() on a Windows host
(fxc.exe), so this script stages shaders_bytes.rs on Windows with the SDK's
fxc.exe and the container copies it into OUT_DIR before building.
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

# Replicates gpui_windows/build.rs compile_shaders() with the SDK's fxc.exe so
# the Linux host (whose build script is cfg'd out) finds shaders_bytes.rs.
function New-StagedShaderBytes {
    param([string]$Fxc, [string]$HlslDir, [string]$OutFile)
    if (-not (Test-Path -LiteralPath $Fxc)) { throw "fxc.exe not found at $Fxc" }
    if (-not (Test-Path -LiteralPath $OutFile -PathType Container)) {
        New-Item -ItemType Directory -Path (Split-Path -Parent $OutFile) -Force | Out-Null
    }
    $entries = @('quad', 'shadow', 'path_rasterization', 'path_sprite', 'underline',
        'monochrome_sprite', 'subpixel_sprite', 'polychrome_sprite') |
        ForEach-Object { @{ Module = $_; File = 'shaders.hlsl' } }
    $entries += @{ Module = 'emoji_rasterization'; File = 'color_text_raster.hlsl' }
    $targets = @(@{ Suf = '_vertex'; Kind = 'VERTEX'; Type = 'vs_4_1' },
        @{ Suf = '_fragment'; Kind = 'FRAGMENT'; Type = 'ps_4_1' })
    $tmp = Join-Path $env:TEMP "xemnas-fxc-$([guid]::NewGuid().ToString('N').Substring(0, 8))"
    New-Item -ItemType Directory -Path $tmp -Force | Out-Null
    if (Test-Path -LiteralPath $OutFile) { Remove-Item -LiteralPath $OutFile -Force }
    try {
        foreach ($e in $entries) {
            $src = Join-Path $HlslDir $e.File
            foreach ($t in $targets) {
                $entry = "$($e.Module)$($t.Suf)"
                $const = "$($e.Module.ToUpper())_$($t.Kind)_BYTES"
                $hdr = Join-Path $tmp "$entry.h"
                # PS 5.1 turns native stderr into a terminating error under
                # $ErrorActionPreference='Stop'; fxc emits warnings on stderr.
                $eap = $ErrorActionPreference
                $ErrorActionPreference = 'Continue'
                try {
                    & $Fxc /nologo /T $t.Type /E $entry /Fh $hdr /Vn $const /O3 $src 2>$null | Out-Null
                    $fxcExit = $LASTEXITCODE
                } finally {
                    $ErrorActionPreference = $eap
                }
                if ($fxcExit -ne 0) { throw "fxc failed for shader entry $entry" }
                $h = Get-Content -Raw -LiteralPath $hdr
                $i = $h.IndexOf('const BYTE')
                if ($i -lt 0) { throw "const BYTE missing in header for $entry" }
                $rest = $h.Substring($i)
                $eq = $rest.IndexOf('=')
                if ($eq -lt 0) { throw "unexpected fxc header for $entry" }
                $val = $rest.Substring($eq + 1).Trim().Replace('{', '[').Replace('}', ']')
                Add-Content -LiteralPath $OutFile -Value "const ${const}: &[u8] = &$val"
            }
        }
    } finally {
        Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
    if (-not (Test-Path -LiteralPath $OutFile)) { throw 'staged shaders_bytes.rs was not created' }
}
$fxcPath = Join-Path $SdkRoot "bin/$sdkVersion/x64/fxc.exe"
$stagedShaders = Join-Path $outputDir 'staged/shaders_bytes.rs'
$gpuiSrc = Get-ChildItem -LiteralPath (Join-Path $env:USERPROFILE '.cargo/git/checkouts') -Directory -Filter 'zed-*' |
    ForEach-Object { Get-ChildItem -LiteralPath $_.FullName -Directory } |
    ForEach-Object { Join-Path $_.FullName 'crates/gpui_windows/src' } |
    Where-Object { Test-Path -LiteralPath (Join-Path $_ 'shaders.hlsl') } |
    Sort-Object -Descending | Select-Object -First 1
if (-not $gpuiSrc) { throw 'gpui_windows source checkout with shaders.hlsl not found.' }
New-StagedShaderBytes -Fxc $fxcPath -HlslDir $gpuiSrc -OutFile $stagedShaders
Write-Output "Staged shaders: $stagedShaders"

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
    & docker cp "${containerName}:/build/x86_64-pc-windows-msvc/release/xemnas.exe" $outputExe
    if ($LASTEXITCODE -ne 0) { throw 'Could not export the compiled executable.' }
    Write-Output "Compiled: $outputExe"
    Write-Output "SHA256: $((Get-FileHash -LiteralPath $outputExe -Algorithm SHA256).Hash)"
} finally {
    & docker rm $containerName | Out-Null
}
