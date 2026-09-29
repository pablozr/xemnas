<#
.SYNOPSIS
    Captures a screenshot of a running xemnas window (app shell or gallery).

.DESCRIPTION
    Launches the given executable, waits for its real top-level window, brings it
    to the foreground and copies the window rect to a PNG. Unlike
    `capture-quiet-glass.ps1` this script asserts nothing about a11y: it exists
    purely to look at the current appearance while iterating on the design.

    Output lands in %TEMP%\xemnas\shots\<name>.png.

.EXAMPLE
    powershell -File tools\capture-window.ps1 -ExePath target\debug\xemnas.exe -Name before
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ExePath,

    [Parameter(Mandatory = $true)]
    [string]$Name,

    # Extra keystrokes to send after the window is up (SendKeys syntax).
    [string]$Keys = '',

    [int]$SettleMs = 2500
)

$ErrorActionPreference = 'Stop'
$ExePath = (Resolve-Path -LiteralPath $ExePath).Path
$shotDir = Join-Path $env:TEMP 'xemnas\shots'
New-Item -ItemType Directory -Force -Path $shotDir | Out-Null
$outPath = Join-Path $shotDir "$Name.png"

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public struct XemnasShotRect { public int Left; public int Top; public int Right; public int Bottom; }
public static class XemnasShotWin {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, ref XemnasShotRect r);
    [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] static extern bool BringWindowToTop(IntPtr h);
    [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, IntPtr p);
    [DllImport("kernel32.dll")] static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] static extern bool AttachThreadInput(uint a, uint b, bool f);
    [DllImport("user32.dll")] static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] static extern bool ShowWindow(IntPtr h, int c);
    [DllImport("user32.dll")] static extern bool IsIconic(IntPtr h);

    public static void Activate(IntPtr hWnd) {
        if (IsIconic(hWnd)) ShowWindow(hWnd, 9);
        keybd_event(0x12, 0, 0, UIntPtr.Zero);
        keybd_event(0x12, 0, 2, UIntPtr.Zero);
        uint fg = GetWindowThreadProcessId(GetForegroundWindow(), IntPtr.Zero);
        uint me = GetCurrentThreadId();
        bool attached = fg != 0 && fg != me && AttachThreadInput(me, fg, true);
        SetForegroundWindow(hWnd);
        BringWindowToTop(hWnd);
        if (attached) AttachThreadInput(me, fg, false);
    }
}
'@

$proc = Start-Process -FilePath $ExePath -PassThru
$sw = [Diagnostics.Stopwatch]::StartNew()
while ($sw.ElapsedMilliseconds -lt 60000) {
    $proc.Refresh()
    if ($proc.MainWindowHandle -ne [IntPtr]::Zero) { break }
    Start-Sleep -Milliseconds 50
}
if ($proc.MainWindowHandle -eq [IntPtr]::Zero) {
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    throw "no top-level window appeared for $ExePath"
}
Start-Sleep -Milliseconds $SettleMs
[XemnasShotWin]::Activate($proc.MainWindowHandle)
Start-Sleep -Milliseconds 600

if ($Keys) {
    [System.Windows.Forms.SendKeys]::SendWait($Keys)
    Start-Sleep -Milliseconds 600
}

$rect = New-Object XemnasShotRect
if (-not [XemnasShotWin]::GetWindowRect($proc.MainWindowHandle, [ref]$rect)) {
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    throw 'GetWindowRect failed'
}
$l = [Math]::Min($rect.Left, $rect.Right)
$t = [Math]::Min($rect.Top, $rect.Bottom)
$r = [Math]::Max($rect.Left, $rect.Right)
$b = [Math]::Max($rect.Top, $rect.Bottom)
$bounds = [System.Drawing.Rectangle]::FromLTRB($l, $t, $r, $b)
$bmp = New-Object System.Drawing.Bitmap($bounds.Width, $bounds.Height)
try {
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    try { $g.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size) }
    finally { $g.Dispose() }
    $bmp.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)
} finally {
    $bmp.Dispose()
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
}

Write-Host "SHOT $outPath ($($bounds.Width)x$($bounds.Height))"
exit 0
