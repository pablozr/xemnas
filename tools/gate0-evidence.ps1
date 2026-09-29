<#
.SYNOPSIS
    Gate 0 evidence collector for the xemnas GPUI spike.

.DESCRIPTION
    Launches the spike executable and records the evidence Gate 0 asks for:
      * time from process start to a usable window (startup)
      * working-set memory once the window is up
      * the Windows UI Automation tree the window exposes — this is what
        Narrator/assistive technology consumes, so it is the accessibility gate
      * screenshots of the resting window and of the focus-visible state after
        a real Tab press (the no-blur fallback is visible in both)
      * the local API rejects an unauthenticated capture request and accepts an
        authenticated one

    Exits 0 only when every assertion holds. Prints a `GATE0 ...` line per
    measurement so the output can be pasted straight into the ADR.

.EXAMPLE
    powershell -File tools\gate0-evidence.ps1 -ExePath ..\xemnas-spike\target\release\xemnas-spike.exe
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
$runtimeDir = Join-Path $env:TEMP 'xemnas-spike'
$discoveryPath = Join-Path $runtimeDir 'discovery.json'
$tokenPath = Join-Path $runtimeDir 'token'
Remove-Item -LiteralPath $discoveryPath, $tokenPath -ErrorAction SilentlyContinue

Write-Host "GATE0 exe=$ExePath"
$sw = [Diagnostics.Stopwatch]::StartNew()
$proc = Start-Process -FilePath $ExePath -PassThru

# --- startup: process start -> local API discoverable -----------------------
while (-not (Test-Path -LiteralPath $discoveryPath) -and $sw.ElapsedMilliseconds -lt 60000) {
    Start-Sleep -Milliseconds 25
}
$discoveryMs = $sw.ElapsedMilliseconds

# --- startup: process start -> a real top-level window ----------------------
$windowMs = $null
while ($sw.ElapsedMilliseconds -lt 60000) {
    $proc.Refresh()
    if ($proc.MainWindowHandle -ne [IntPtr]::Zero) { $windowMs = $sw.ElapsedMilliseconds; break }
    Start-Sleep -Milliseconds 25
}

$discovery = if (Test-Path -LiteralPath $discoveryPath) {
    Get-Content -LiteralPath $discoveryPath -Raw -Encoding UTF8 | ConvertFrom-Json
} else { $null }
$token = if (Test-Path -LiteralPath $tokenPath) {
    (Get-Content -LiteralPath $tokenPath -Raw -Encoding UTF8).Trim()
} else { $null }

Write-Host "GATE0 startup_to_discovery=$discoveryMs ms"
Assert ($null -ne $discovery) 'local discovery file was written'
Assert ($windowMs -ne $null) 'the window handle appeared'
Write-Host "GATE0 startup_to_window=$windowMs ms"

# Give the first frame a moment, then measure steady-state memory.
Start-Sleep -Seconds 2
$proc.Refresh()
$memoryKb = [math]::Round($proc.WorkingSet64 / 1KB)
Write-Host "GATE0 working_set=$memoryKb KB"

# --- local API: no token must fail, token must succeed ----------------------
$port = if ($discovery) { [int]$discovery.port } else { 0 }
if ($port -gt 0) {
    # Bodies go through files so the JSON quotes never have to survive
    # PowerShell -> native command-line re-quoting. The idempotency key is
    # per-run so every evidence run really inserts a row.
    $stamp = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    $noTokenBody = Join-Path $env:TEMP 'gate0-body-anon.json'
    $withTokenBody = Join-Path $env:TEMP 'gate0-body-token.json'
    [IO.File]::WriteAllText($noTokenBody,
        ('{{"id":"gate0","idempotency_key":"gate0-no-token-{0}","project_path":"C:/projects/xemnas","observed_at":"2026-09-28T00:00:00.000Z"}}' -f $stamp),
        [Text.UTF8Encoding]::new($false))
    [IO.File]::WriteAllText($withTokenBody,
        ('{{"id":"gate0","idempotency_key":"gate0-with-token-{0}","project_path":"C:/projects/xemnas","observed_at":"2026-09-28T00:00:00.000Z"}}' -f $stamp),
        [Text.UTF8Encoding]::new($false))

    $anon = & curl.exe -s -o NUL -w "%{http_code}" -X POST `
        -H 'Content-Type: application/json' --data-binary "@$noTokenBody" `
        "http://127.0.0.1:$port/v1/captures"
    Write-Host "GATE0 api_post_without_token=$anon"
    Assert ($anon -eq '401') 'POST /v1/captures without a bearer token is rejected with 401'

    $auth = & curl.exe -s -o NUL -w "%{http_code}" -X POST `
        -H 'Content-Type: application/json' -H "Authorization: Bearer $token" `
        --data-binary "@$withTokenBody" `
        "http://127.0.0.1:$port/v1/captures"
    Write-Host "GATE0 api_post_with_token=$auth"
    Assert ($auth -eq '200') 'POST /v1/captures with the bearer token is accepted'

    $health = & curl.exe -s -o NUL -w "%{http_code}" "http://127.0.0.1:$port/v1/health"
    Write-Host "GATE0 api_health=$health"
    Assert ($health -eq '200') 'GET /v1/health answers on loopback'

    Remove-Item -LiteralPath $noTokenBody, $withTokenBody -ErrorAction SilentlyContinue
}
else {
    Assert $false 'discovery file carried a port'
}

# --- UI Automation tree: what Narrator actually reads -----------------------
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

$root = [System.Windows.Automation.AutomationElement]::RootElement
$byPid = New-Object System.Windows.Automation.PropertyCondition(
    [System.Windows.Automation.AutomationElement]::ProcessIdProperty, $proc.Id)
$window = $root.FindFirst([System.Windows.Automation.TreeScope]::Children, $byPid)

Assert ($null -ne $window) 'UI Automation can see the spike window'
if ($window) {
    Write-Host "GATE0 window_name=$($window.Current.Name)"

    function Get-UiA11yTree([System.Windows.Automation.AutomationElement]$element) {
        $list = New-Object System.Collections.Generic.List[object]
        $stack = New-Object System.Collections.Generic.Stack[System.Windows.Automation.AutomationElement]
        $stack.Push($element)
        $guard = 0
        while ($stack.Count -gt 0 -and $guard -lt 20000) {
            $guard++
            $node = $stack.Pop()
            $cur = $node.Current
            $list.Add([pscustomobject]@{
                ControlType         = ($cur.ControlType.ProgrammaticName -replace 'ControlType\.', '')
                Name                = $cur.Name
                AutomationId        = $cur.AutomationId
                IsKeyboardFocusable = $cur.IsKeyboardFocusable
            })
            foreach ($child in $node.FindAll(
                [System.Windows.Automation.TreeScope]::Children,
                [System.Windows.Automation.Condition]::TrueCondition)) {
                $stack.Push($child)
            }
        }
        return $list
    }

    # AccessKit only starts publishing a tree once a client attaches, so the first
    # walk warms the provider up and the second one is the reading that counts.
    $null = Get-UiA11yTree $window
    Start-Sleep -Milliseconds 1500
    $walked = Get-UiA11yTree $window

    Write-Host "GATE0 ui_nodes=$($walked.Count)"
    $walked | ForEach-Object {
        Write-Host ("  {0,-12} focusable={1,-5} {2}" -f $_.ControlType, $_.IsKeyboardFocusable, $_.Name)
    }

    $types = @($walked | ForEach-Object { $_.ControlType } | Sort-Object -Unique)
    $names = @($walked | ForEach-Object { $_.Name })
    $joined = $names -join '|'

    Assert ($walked.Count -gt 5) 'the accessibility tree has more than a bare window node'
    Assert (@($types) -contains 'Button') 'the tree exposes Button controls'
    Assert ($names -contains 'Confirm decision') 'the tree exposes the Confirm action by name'
    Assert ($names -contains 'Adjust decision') 'the tree exposes the Adjust action by name'
    Assert ($joined -like '*Decision Inbox*') 'the tree exposes the inbox region by name'
    Assert ($joined -match 'Decision Inbox, [1-9][0-9]* receipts') 'the background sampler refreshed the receipt count shown in the UI'
    Assert (@($types) -contains 'ListItem' -or @($types) -contains 'List') 'the tree exposes the inbox list'
    Assert (@($walked | Where-Object { $_.IsKeyboardFocusable }).Count -gt 3) 'several elements are keyboard focusable'
    Assert ($joined -like '*Inbox*') 'the tree exposes navigation items by name'

    # --- screenshots: focus-visible and the no-blur fallback -----------------
    # Gate 0 also asks for a *visual* demonstration of keyboard focus and of the
    # Quiet Glass surfaces that never depend on a backdrop blur, so record the
    # window before and after a real Tab press.
    $evidenceDir = Join-Path $env:TEMP 'xemnas-spike\evidence'
    New-Item -ItemType Directory -Force -Path $evidenceDir | Out-Null

    Add-Type -AssemblyName System.Drawing
    Add-Type -AssemblyName System.Windows.Forms

    function Save-WindowScreenshot([System.Windows.Automation.AutomationElement]$element, [string]$path) {
        $r = $element.Current.BoundingRectangle
        if ($r.Width -le 0 -or $r.Height -le 0) { return $null }
        $rect = [System.Drawing.Rectangle]::FromLTRB([int]$r.Left, [int]$r.Top, [int]$r.Right, [int]$r.Bottom)
        $bmp = New-Object System.Drawing.Bitmap($rect.Width, $rect.Height)
        try {
            $g = [System.Drawing.Graphics]::FromImage($bmp)
            try { $g.CopyFromScreen($rect.Location, [System.Drawing.Point]::Empty, $rect.Size) }
            finally { $g.Dispose() }
            $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
            return $path
        } catch { return $null } finally { $bmp.Dispose() }
    }

    $idleShot = Save-WindowScreenshot $window (Join-Path $evidenceDir 'window-idle.png')
    Assert ($null -ne $idleShot) 'the Quiet Glass window was screenshotted'
    if ($idleShot) { Write-Host "GATE0 screenshot_idle=$idleShot" }

    # Tab into the app and let focus-visible paint. AccessKit forwards keyboard
    # input, so the focused element is what Narrator would follow. The top-level
    # window is not focusable, so activate it first: Windows blocks
    # SetForegroundWindow from a background process unless the calling thread
    # borrows the current foreground thread's input queue. Something else on the
    # desktop can also reclaim focus a few hundred milliseconds later, so both
    # the Tab and the focus read have to happen inside that window.
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class XemnasForeground {
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
        // A synthetic VK_MENU press makes this process eligible to take the
        // foreground again, which is otherwise refused to background processes.
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
    $activated = [XemnasForeground]::Activate($proc.MainWindowHandle)
    Write-Host "GATE0 window_activated=$activated"

    $deadline = [DateTime]::UtcNow.AddSeconds(3)
    $ownsForeground = $false
    while ([DateTime]::UtcNow -lt $deadline -and -not $ownsForeground) {
        $ownsForeground = [XemnasForeground]::IsForeground($proc.MainWindowHandle)
        if (-not $ownsForeground) { Start-Sleep -Milliseconds 30 }
    }

    [System.Windows.Forms.SendKeys]::SendWait("{TAB}")

    $focusedName = ''
    $focusedOk = $false
    $deadline = [DateTime]::UtcNow.AddSeconds(3)
    while ([DateTime]::UtcNow -lt $deadline -and -not $focusedOk) {
        $focused = [System.Windows.Automation.AutomationElement]::FocusedElement
        if ($focused -and $focused.Current.ProcessId -eq $proc.Id) {
            $focusedName = $focused.Current.Name
            $focusedOk = $true
            break
        }
        Start-Sleep -Milliseconds 30
    }
    Write-Host "GATE0 owns_foreground=$ownsForeground focused=$focusedName"

    # Screenshot inside the same focus window: focus-visible only paints while
    # the window actually owns keyboard focus.
    $focusShot = Save-WindowScreenshot $window (Join-Path $evidenceDir 'window-focus-visible.png')
    Assert ($null -ne $focusShot) 'the focus-visible state was screenshotted'
    if ($focusShot) { Write-Host "GATE0 screenshot_focus=$focusShot" }
    Assert $focusedOk "Tab moved keyboard focus into the spike window (focused=$focusedName)"
}

# --- done -------------------------------------------------------------------
if (-not $KeepRunning) {
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    $proc.WaitForExit()
}
else {
    Write-Host "GATE0 left running pid=$($proc.Id)"
}

if ($failures.Count -gt 0) {
    Write-Host "GATE0 RESULT=FAIL ($($failures.Count) assertion(s))" -ForegroundColor Red
    $failures | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit 1
}
Write-Host 'GATE0 RESULT=PASS'
exit 0
