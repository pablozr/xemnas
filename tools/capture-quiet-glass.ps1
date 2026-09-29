<#
.SYNOPSIS
    Quiet Glass gallery evidence collector (Gate 1, ticket 05).

.DESCRIPTION
    Launches the `quiet-glass-gallery` binary at 1440x1024 and records the
    screenshots the ticket asks for:
      * the resting gallery with every primitive in its documented states
        (normal, hover, foco, disabled, loading, vazio, erro);
      * the same window after a real Tab press, which paints the focus-visible
        ring (the ring is never removed for aesthetics);
      * the Windows UI Automation tree, proving each control has an accessible
        name (Narrator consumes this tree).

    Screenshots are written to %TEMP%\xemnas\evidence\05-*.png. `docs/**` is the
    planner's to edit, so this script never writes there; copy the files by hand
    once the canonical reference image is restored.

    Exits 0 only when every assertion holds. Prints `QUIETGLASS ...` lines.

.EXAMPLE
    powershell -File tools\capture-quiet-glass.ps1 -ExePath apps\desktop-gpui\target\debug\quiet-glass-gallery.exe
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ExePath,

    # Leave the app running after the checks (useful for a manual Narrator pass).
    [switch]$KeepRunning
)

# UI Automation is an STA API; re-exec if this session is MTA.
if ([Threading.Thread]::CurrentThread.ApartmentState -ne 'STA') {
    $reexec = @('-NoProfile', '-STA', '-File', $PSCommandPath)
    foreach ($k in 'ExePath') { $reexec += @("-$k", (Get-Variable -Name $k -ValueOnly)) }
    if ($KeepRunning) { $reexec += '-KeepRunning' }
    & (Join-Path $PSHOME 'powershell.exe') @reexec
    exit $LASTEXITCODE
}

$ErrorActionPreference = 'Stop'
$failures = New-Object System.Collections.Generic.List[string]

function Assert([bool]$Condition, [string]$Message) {
    if ($Condition) { Write-Host "  ok   $Message" }
    else { $failures.Add($Message); Write-Host "  FAIL $Message" -ForegroundColor Red }
}

$ExePath = (Resolve-Path -LiteralPath $ExePath).Path
$evidenceDir = Join-Path $env:TEMP 'xemnas\evidence'
New-Item -ItemType Directory -Force -Path $evidenceDir | Out-Null

Write-Host "QUIETGLASS exe=$ExePath"
$proc = Start-Process -FilePath $ExePath -PassThru

# --- wait for a real top-level window ---------------------------------------
$sw = [Diagnostics.Stopwatch]::StartNew()
while ($sw.ElapsedMilliseconds -lt 60000) {
    $proc.Refresh()
    if ($proc.MainWindowHandle -ne [IntPtr]::Zero) { break }
    Start-Sleep -Milliseconds 50
}
Assert ($proc.MainWindowHandle -ne [IntPtr]::Zero) 'the gallery window appeared'
Start-Sleep -Seconds 2

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class XemnasQuietGlassForeground {
    [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] static extern bool BringWindowToTop(IntPtr h);
    [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, IntPtr p);
    [DllImport("kernel32.dll")] static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] static extern bool AttachThreadInput(uint a, uint b, bool f);
    [DllImport("user32.dll")] static extern bool ShowWindow(IntPtr h, int c);
    [DllImport("user32.dll")] static extern bool IsIconic(IntPtr h);
    [DllImport("user32.dll")] static extern void SwitchToThisWindow(IntPtr h, bool fUnknown);
    [DllImport("user32.dll")] static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);

    public static bool IsForeground(IntPtr hWnd) { return GetForegroundWindow() == hWnd; }

    public static bool Activate(IntPtr hWnd) {
        if (IsIconic(hWnd)) ShowWindow(hWnd, 9); // SW_RESTORE
        keybd_event(0x12, 0, 0, UIntPtr.Zero);
        keybd_event(0x12, 0, 2, UIntPtr.Zero);
        uint fg = GetWindowThreadProcessId(GetForegroundWindow(), IntPtr.Zero);
        uint me = GetCurrentThreadId();
        bool attached = fg != 0 && fg != me && AttachThreadInput(me, fg, true);
        bool ok = SetForegroundWindow(hWnd);
        SwitchToThisWindow(hWnd, true);
        BringWindowToTop(hWnd);
        if (attached) AttachThreadInput(me, fg, false);
        return ok;
    }
}
'@

# The window rect is fetched through a small P/Invoke helper so the screenshot
# does not depend on the UI Automation element surviving a focus change.
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public struct XemnasQuietGlassRect { public int Left; public int Top; public int Right; public int Bottom; }
public static class XemnasQuietGlassWindow {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, ref XemnasQuietGlassRect r);
}
'@

function Save-WindowScreenshot([IntPtr]$handle, [string]$path) {
    $rect = New-Object XemnasQuietGlassRect
    if (-not [XemnasQuietGlassWindow]::GetWindowRect($handle, [ref]$rect)) { return $null }
    $l = [Math]::Min($rect.Left, $rect.Right)
    $t = [Math]::Min($rect.Top, $rect.Bottom)
    $r = [Math]::Max($rect.Left, $rect.Right)
    $b = [Math]::Max($rect.Top, $rect.Bottom)
    if (($r - $l) -le 0 -or ($b - $t) -le 0) { return $null }
    $bounds = [System.Drawing.Rectangle]::FromLTRB($l, $t, $r, $b)
    $bmp = New-Object System.Drawing.Bitmap($bounds.Width, $bounds.Height)
    try {
        $graphics = [System.Drawing.Graphics]::FromImage($bmp)
        try { $graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size) }
        finally { $graphics.Dispose() }
        $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
        return $path
    } catch { return $null } finally { $bmp.Dispose() }
}

# --- UI Automation: accessible names on every control -----------------------
$root = [System.Windows.Automation.AutomationElement]::RootElement
$byPid = New-Object System.Windows.Automation.PropertyCondition(
    [System.Windows.Automation.AutomationElement]::ProcessIdProperty, $proc.Id)
$window = $root.FindFirst([System.Windows.Automation.TreeScope]::Children, $byPid)
Assert ($null -ne $window) 'UI Automation can see the gallery window'

if ($window) {
    function Get-UiA11yNames([System.Windows.Automation.AutomationElement]$element) {
        $names = New-Object System.Collections.Generic.List[string]
        $stack = New-Object System.Collections.Generic.Stack[System.Windows.Automation.AutomationElement]
        $stack.Push($element)
        $guard = 0
        while ($stack.Count -gt 0 -and $guard -lt 20000) {
            $guard++
            $node = $stack.Pop()
            $names.Add([string]$node.Current.Name)
            foreach ($child in $node.FindAll(
                [System.Windows.Automation.TreeScope]::Children,
                [System.Windows.Automation.Condition]::TrueCondition)) {
                $stack.Push($child)
            }
        }
        return $names
    }

    # AccessKit only starts publishing a tree once a client attaches, so warm it
    # up first and read on the second walk.
    $null = Get-UiA11yNames $window
    Start-Sleep -Milliseconds 1500
    $names = Get-UiA11yNames $window
    $joined = $names -join '|'
    Write-Host "QUIETGLASS ui_nodes=$($names.Count)"

    foreach ($expected in @(
        'Confirmar decis',
        'Ajustar',
        'Rejeitar',
        'Copiar',
        'Buscar decis',
        'Quiet Glass'
    )) {
        Assert ($joined -like "*$expected*") "the tree exposes an accessible name matching '$expected'"
    }

    # --- screenshots: resting gallery and focus-visible ----------------------
    $idle = Save-WindowScreenshot $proc.MainWindowHandle (Join-Path $evidenceDir '05-gallery-idle.png')
    Assert ($null -ne $idle) 'the resting gallery was screenshotted'
    if ($idle) { Write-Host "QUIETGLASS screenshot_idle=$idle" }

    $activated = [XemnasQuietGlassForeground]::Activate($proc.MainWindowHandle)
    Write-Host "QUIETGLASS window_activated=$activated"
    $deadline = [DateTime]::UtcNow.AddSeconds(3)
    $ownsForeground = $false
    while ([DateTime]::UtcNow -lt $deadline -and -not $ownsForeground) {
        $ownsForeground = [XemnasQuietGlassForeground]::IsForeground($proc.MainWindowHandle)
        if (-not $ownsForeground) { Start-Sleep -Milliseconds 30 }
    }
    Assert ($ownsForeground) 'the gallery owns the foreground before Tab'
    [System.Windows.Forms.SendKeys]::SendWait("{TAB}")
    Start-Sleep -Milliseconds 400
    $focusShot = Save-WindowScreenshot $proc.MainWindowHandle (Join-Path $evidenceDir '05-gallery-focus-visible.png')
    Assert ($null -ne $focusShot) 'the focus-visible state was screenshotted'
    if ($focusShot) { Write-Host "QUIETGLASS screenshot_focus=$focusShot" }

    # Prove the Tab really changed the rendered frame. If the resting and
    # focus-visible captures are identical inside the client area the focus
    # evidence is false, so the run must fail instead of passing silently.
    if ($idle -and $focusShot) {
        $bitmapIdle = [System.Drawing.Bitmap]::FromFile($idle)
        $bitmapFocus = [System.Drawing.Bitmap]::FromFile($focusShot)
        try {
            $left = 12
            $top = 40
            $right = [Math]::Min($bitmapIdle.Width, $bitmapFocus.Width) - 12
            $bottom = [Math]::Min($bitmapIdle.Height, $bitmapFocus.Height) - 20
            $changed = 0
            for ($y = $top; $y -lt $bottom; $y++) {
                for ($x = $left; $x -lt $right; $x++) {
                    if ($bitmapIdle.GetPixel($x, $y).ToArgb() -ne $bitmapFocus.GetPixel($x, $y).ToArgb()) {
                        $changed++
                    }
                }
            }
            Write-Host "QUIETGLASS focus_pixel_diff=$changed"
            Assert ($changed -gt 0) 'Tab changed the rendered frame (real focus-visible evidence)'
        } finally {
            $bitmapIdle.Dispose()
            $bitmapFocus.Dispose()
        }
    }
}

if (-not $KeepRunning) {
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    $proc.WaitForExit()
} else {
    Write-Host "QUIETGLASS left running pid=$($proc.Id)"
}

if ($failures.Count -gt 0) {
    Write-Host "QUIETGLASS RESULT=FAIL ($($failures.Count) assertion(s))" -ForegroundColor Red
    $failures | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit 1
}
Write-Host 'QUIETGLASS RESULT=PASS'
exit 0
