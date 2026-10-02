<#
.SYNOPSIS
    Captures a demo screen of xemnas without focus, clicks or keys.

.DESCRIPTION
    Starts the binary with `--demo --background`, which opens the window off
    every monitor and without taking focus, reaches the screen through
    `--open <route>` and copies the window with PrintWindow
    (PW_RENDERFULLCONTENT). Nothing on screen is covered and no input is
    sent, so it is safe while the machine is in use.

    Routes: overview, overview:flow0, review, decisions, context, map,
    map:timeline, map:suggestions, map:file, map:entity:<name>.

.EXAMPLE
    powershell -File tools\capture-background.ps1 -Route map:timeline -Name timeline
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Route,
    [Parameter(Mandatory = $true)][string]$Name,
    [string]$ExePath = "target\debug\xemnas.exe",
    [ValidateSet('quiet', 'charcoal', 'organization', 'moss', 'midnight')][string]$Theme = 'quiet',
    [string]$Wallpaper = '',
    [switch]$Compact,
    [int]$SettleMs = 3500
)

$ErrorActionPreference = 'Stop'
$ExePath = (Resolve-Path -LiteralPath $ExePath).Path
$shotDir = Join-Path $env:TEMP 'xemnas\shots'
New-Item -ItemType Directory -Force -Path $shotDir | Out-Null
$outPath = Join-Path $shotDir "$Name.png"

Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public struct XemnasBgRect { public int Left; public int Top; public int Right; public int Bottom; }
public static class XemnasBg {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, ref XemnasBgRect r);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int width, int height, uint flags);
}
'@

$arguments = @('--demo', '--background', '--open', $Route)
if ($Theme -ne 'quiet') { $arguments += @('--theme', $Theme) }
if ($Wallpaper -ne '') { $arguments += @('--wallpaper', $Wallpaper) }
if ($Compact) { $arguments += '--compact' }
$proc = Start-Process -FilePath $ExePath -ArgumentList $arguments -PassThru
try {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($sw.ElapsedMilliseconds -lt 60000) {
        $proc.Refresh()
        if ($proc.MainWindowHandle -ne [IntPtr]::Zero) { break }
        Start-Sleep -Milliseconds 50
    }
    if ($proc.MainWindowHandle -eq [IntPtr]::Zero) { throw "no window appeared" }
    if ($Compact) {
        # SWP_NOZORDER | SWP_NOACTIVATE: resize offscreen without focus or input.
        if (-not [XemnasBg]::SetWindowPos($proc.MainWindowHandle, [IntPtr]::Zero, -12000, 0, 1180, 760, 0x14)) {
            throw "background compact resize failed"
        }
    }
    Start-Sleep -Milliseconds $SettleMs

    $rect = New-Object XemnasBgRect
    [void][XemnasBg]::GetWindowRect($proc.MainWindowHandle, [ref]$rect)
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    $bmp = New-Object System.Drawing.Bitmap($width, $height)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $hdc = $g.GetHdc()
    $ok = [XemnasBg]::PrintWindow($proc.MainWindowHandle, $hdc, 2)
    $g.ReleaseHdc($hdc)
    $g.Dispose()
    if (-not $ok) { throw "PrintWindow failed" }
    $bmp.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Host "SHOT $outPath (${width}x${height})"
} finally {
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
}
