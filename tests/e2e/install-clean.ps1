<#
.SYNOPSIS
    End-to-end evidence for ticket 18: clean install and clean uninstall.

.DESCRIPTION
    Packages the release build (`tools\package.ps1`), extracts the ZIP into a
    fresh temporary directory, launches the installed `xemnas.exe` with an
    isolated `XEMNAS_DATA_DIR`, waits for `GET /v1/health` to answer `ok` using
    the port from `discovery.json` and the bearer token from `api-token`,
    verifies `schema_migrations` reached version 10, closes the window gracefully,
    and then uninstalls by removing the extraction directory and the data dir.

    Writes RESULT=PASS/FAIL and exits 0/1. Pure ASCII on purpose.

.PARAMETER ExePath
    Use an existing binary instead of packaging; it is copied into the temporary
    install directory. Implies -SkipPackage.

.PARAMETER SkipPackage
    Do not run `tools\package.ps1`; requires -ExePath.

.EXAMPLE
    powershell -NoProfile -File tests\e2e\install-clean.ps1
#>
[CmdletBinding()]
param(
    [string]$ExePath,
    [switch]$SkipPackage
)

$ErrorActionPreference = 'Stop'

# cargo lives under the user profile in this environment.
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

$failures = New-Object System.Collections.Generic.List[string]

function Assert([bool]$Condition, [string]$Message) {
    if ($Condition) { Write-Host "  ok   $Message" }
    else { $failures.Add($Message); Write-Host "  FAIL $Message" -ForegroundColor Red }
}

# Runs a native command whose stderr output must not become a terminating error.
function Invoke-Native([scriptblock]$Command) {
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & $Command *> $null
        return $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previous
    }
}

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class XemnasWindowsInstall {
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] static extern bool PostMessage(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam);
    delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    public static IntPtr FindTopLevelWindow(uint pid) {
        IntPtr found = IntPtr.Zero;
        EnumWindows(delegate(IntPtr h, IntPtr l) {
            uint p = 0;
            GetWindowThreadProcessId(h, out p);
            if (p == pid && IsWindowVisible(h)) { found = h; return false; }
            return true;
        }, IntPtr.Zero);
        return found;
    }
    public static bool CloseWindow(IntPtr hWnd) {
        return PostMessage(hWnd, 0x0010, IntPtr.Zero, IntPtr.Zero);
    }
}
'@

$python = Get-Command python -ErrorAction SilentlyContinue
if (-not $python) { throw 'python is required for the schema version check' }

# --- package or reuse an existing binary -----------------------------------

$installDir = Join-Path $env:TEMP ("xemnas-install-clean-" + [Guid]::NewGuid().ToString('N'))
$dataDir = Join-Path $env:TEMP ("xemnas-install-data-" + [Guid]::NewGuid().ToString('N'))
$stateDir = Join-Path $dataDir 'state'
New-Item -ItemType Directory -Force -Path $installDir, $stateDir | Out-Null

if ($ExePath) {
    $source = (Resolve-Path -LiteralPath $ExePath).Path
    Copy-Item -LiteralPath $source -Destination (Join-Path $installDir 'xemnas.exe') -Force
} elseif ($SkipPackage) {
    throw '-SkipPackage requires -ExePath'
} else {
    Write-Host 'INSTALL-E2E packaging'
    $zip = (& (Join-Path $repoRoot 'tools\package.ps1') | Select-Object -Last 1)
    if ($LASTEXITCODE -ne 0) { throw "package.ps1 failed with exit $LASTEXITCODE" }
    $zip = (Resolve-Path -LiteralPath $zip).Path
    Assert (Test-Path -LiteralPath $zip) "the archive exists ($(Split-Path -Leaf $zip))"
    Expand-Archive -LiteralPath $zip -DestinationPath $installDir -Force
}

$installedExe = Join-Path $installDir 'xemnas.exe'
Assert (Test-Path -LiteralPath $installedExe) 'the installed binary exists'

# --- launch with an isolated data dir --------------------------------------

$env:XEMNAS_DATA_DIR = $dataDir
Write-Host "INSTALL-E2E install_dir=$installDir data_dir=$dataDir"
$proc = Start-Process -FilePath $installedExe -PassThru

$health = $null
$window = [IntPtr]::Zero
$graceful = $false
try {
    $discoveryPath = Join-Path $stateDir 'discovery.json'
    $tokenPath = Join-Path $stateDir 'api-token'
    $deadline = (Get-Date).AddSeconds(30)
    while ((Get-Date) -lt $deadline) {
        if ($proc.HasExited) { break }
        $window = [XemnasWindowsInstall]::FindTopLevelWindow([uint32]$proc.Id)
        if ((Test-Path -LiteralPath $discoveryPath) -and (Test-Path -LiteralPath $tokenPath)) {
            try {
                $discovery = Get-Content -LiteralPath $discoveryPath -Raw | ConvertFrom-Json
                $token = (Get-Content -LiteralPath $tokenPath -Raw).Trim()
                $headers = @{ Authorization = "Bearer $token" }
                $health = Invoke-RestMethod -Uri "http://127.0.0.1:$($discovery.port)/v1/health" -Headers $headers -TimeoutSec 5
                if ($health.status -eq 'ok') { break }
            } catch {
                $health = $null
            }
        }
        Start-Sleep -Milliseconds 200
    }

    $proc.Refresh()
    Assert (-not $proc.HasExited) 'the installed app stayed alive'
    Assert ($window -ne [IntPtr]::Zero) 'the installed app opened a visible window'
    Assert ($null -ne $health -and $health.status -eq 'ok') 'GET /v1/health answered ok'

    # --- data dir created with the current schema --------------------------

    $dbPath = Join-Path $stateDir 'app.db'
    Assert (Test-Path -LiteralPath $dbPath) 'the data dir was created with app.db'

    $schemaProgram = @'
import sqlite3, json, sys
con = sqlite3.connect(sys.argv[1])
versions = [row[0] for row in con.execute("SELECT version FROM schema_migrations ORDER BY version")]
con.close()
print(json.dumps({"versions": versions, "max": max(versions) if versions else 0}))
'@
    $schemaPath = Join-Path $dataDir 'schema.py'
    [IO.File]::WriteAllText($schemaPath, $schemaProgram, [Text.UTF8Encoding]::new($false))
    $schema = (& $python.Source $schemaPath $dbPath | Out-String).Trim() | ConvertFrom-Json
    Assert ($schema.max -eq 10) "schema_migrations reached version 10 (max=$($schema.max))"

    # --- graceful shutdown --------------------------------------------------

    if ($window -ne [IntPtr]::Zero) {
        [void][XemnasWindowsInstall]::CloseWindow($window)
    }
    $exitDeadline = (Get-Date).AddSeconds(15)
    while ((Get-Date) -lt $exitDeadline -and -not $proc.HasExited) {
        Start-Sleep -Milliseconds 200
    }
    if ($proc.HasExited) {
        $graceful = $true
    }
    Assert $graceful 'the app closed its window and exited gracefully'
    if ($graceful) {
        Assert (-not (Test-Path -LiteralPath $discoveryPath)) 'graceful shutdown removed discovery.json'
        Assert (-not (Test-Path -LiteralPath $tokenPath)) 'graceful shutdown removed api-token'
    }
} finally {
    if (-not $proc.HasExited) {
        Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    }
    try { $proc.WaitForExit() } catch { }
}

# --- clean uninstall --------------------------------------------------------

Remove-Item -LiteralPath $installDir -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $dataDir -Recurse -Force -ErrorAction SilentlyContinue
Assert (-not (Test-Path -LiteralPath $installDir)) 'uninstall removed the install directory'
Assert (-not (Test-Path -LiteralPath $dataDir)) 'uninstall removed the isolated data directory'

# --- report -----------------------------------------------------------------

if ($failures.Count -gt 0) {
    Write-Host "RESULT=FAIL ($($failures.Count) assertion(s))" -ForegroundColor Red
    $failures | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit 1
}

Write-Host 'RESULT=PASS'
exit 0
