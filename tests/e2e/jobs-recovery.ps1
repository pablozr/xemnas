<#
.SYNOPSIS
    End-to-end evidence for ticket 07: persisted, recoverable jobs.

.DESCRIPTION
    Seeds a fresh XEMNAS_DATA_DIR with three jobs (running+idempotent,
    running+non-idempotent, queued), launches the real desktop binary, waits for
    its window, then kills it and inspects the database. Asserts:
      * the idempotent interrupted job is back to queued;
      * the non-idempotent interrupted job is failed with a diagnostic;
      * the queued job is untouched;
      * the projects table was not modified;
      * the process kept a live window while it ran (it did not die).

    Writes RESULT=PASS/FAIL and exits 0/1. Pure ASCII on purpose.

.EXAMPLE
    powershell -NoProfile -File tests\e2e\jobs-recovery.ps1
#>
[CmdletBinding()]
param(
    [string]$ExePath
)

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
if (-not $ExePath) {
    $ExePath = Join-Path $repoRoot 'target\debug\xemnas.exe'
}
$ExePath = (Resolve-Path -LiteralPath $ExePath).Path

$failures = New-Object System.Collections.Generic.List[string]

function Assert([bool]$Condition, [string]$Message) {
    if ($Condition) { Write-Host "  ok   $Message" }
    else { $failures.Add($Message); Write-Host "  FAIL $Message" -ForegroundColor Red }
}

function Assert-HasPython {
    $python = Get-Command python -ErrorAction SilentlyContinue
    if (-not $python) { throw 'python is required for the jobs E2E seed/query' }
    return $python.Source
}

# Top-level window lookup by PID (EnumWindows), no extra dependency.
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class XemnasWindows {
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hWnd);
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
}
'@

$dataDir = Join-Path $env:TEMP ("xemnas-jobs-e2e-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path (Join-Path $dataDir 'state') | Out-Null
$dbPath = Join-Path $dataDir 'state\app.db'
$env:XEMNAS_DATA_DIR = $dataDir

$python = Assert-HasPython

$seed = @'
import sqlite3, sys
db = sys.argv[1]
con = sqlite3.connect(db)
con.executescript("""
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    location TEXT UNIQUE NOT NULL,
    registered_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS jobs (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    payload TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('queued','running','completed','failed','cancelled')),
    idempotent INTEGER NOT NULL CHECK (idempotent IN (0,1)),
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
""")
con.execute("INSERT INTO projects (id, location, registered_at) VALUES (?, ?, ?)",
            ("seed-project", "C:/work/seed", "2026-01-01T00:00:00Z"))
rows = [
    ("job-idempotent", "analysis", '{"note":"synthetic"}', "running", 1, 1, None,
     "2026-01-01T00:00:00Z", "2026-01-01T00:00:00Z"),
    ("job-non-idempotent", "analysis", '{"note":"synthetic"}', "running", 0, 1, None,
     "2026-01-01T00:00:01Z", "2026-01-01T00:00:01Z"),
    ("job-queued", "analysis", '{"note":"synthetic"}', "queued", 1, 0, None,
     "2026-01-01T00:00:02Z", "2026-01-01T00:00:02Z"),
]
con.executemany("INSERT INTO jobs VALUES (?,?,?,?,?,?,?,?,?)", rows)
con.commit()
con.close()
'@
# The Python program goes to a file: Windows native argument quoting mangles
# the embedded quotes and newlines that `python -c` would receive.
$seedPath = Join-Path $dataDir 'seed.py'
[IO.File]::WriteAllText($seedPath, $seed, [Text.UTF8Encoding]::new($false))
& $python $seedPath $dbPath
if ($LASTEXITCODE -ne 0) { throw "seed failed with exit $LASTEXITCODE" }

Write-Host "JOBS-E2E exe=$ExePath"
$proc = Start-Process -FilePath $ExePath -PassThru

$window = [IntPtr]::Zero
$deadline = (Get-Date).AddSeconds(30)
while ((Get-Date) -lt $deadline) {
    if ($proc.HasExited) { break }
    $window = [XemnasWindows]::FindTopLevelWindow([uint32]$proc.Id)
    if ($window -ne [IntPtr]::Zero) { break }
    Start-Sleep -Milliseconds 100
}

Assert ($window -ne [IntPtr]::Zero) 'the app opened a visible top-level window'
Assert (-not $proc.HasExited) 'the app stayed alive after startup'

# Let the boot-time recovery finish while the window is up.
Start-Sleep -Seconds 3
$proc.Refresh()
Assert (-not $proc.HasExited) 'the app was still running (window live) during recovery'

Stop-Process -Id $proc.Id -Force
$proc.WaitForExit()

$query = @'
import sqlite3, json, sys
db = sys.argv[1]
con = sqlite3.connect(db)
jobs = {}
for row in con.execute("SELECT id, state, last_error FROM jobs"):
    jobs[row[0]] = {"state": row[1], "last_error": row[2]}
projects = con.execute("SELECT COUNT(*) FROM projects").fetchone()[0]
print(json.dumps({"jobs": jobs, "projects": projects}))
'@
$queryPath = Join-Path $dataDir 'query.py'
[IO.File]::WriteAllText($queryPath, $query, [Text.UTF8Encoding]::new($false))
$raw = (& $python $queryPath $dbPath | Out-String).Trim()
if ($LASTEXITCODE -ne 0) { throw "query failed with exit $LASTEXITCODE" }
$state = $raw | ConvertFrom-Json

Assert ($state.jobs.'job-idempotent'.state -eq 'queued') 'interrupted idempotent job returned to queued'
Assert ($state.jobs.'job-non-idempotent'.state -eq 'failed') 'interrupted non-idempotent job was failed'
Assert ((!([string]::IsNullOrEmpty($state.jobs.'job-non-idempotent'.last_error)))) 'failed job carries a diagnostic'
Assert ($state.jobs.'job-queued'.state -eq 'queued') 'untouched queued job stayed queued'
Assert ($state.projects -eq 1) 'projects table was not modified'

Remove-Item -LiteralPath $dataDir -Recurse -Force -ErrorAction SilentlyContinue

if ($failures.Count -gt 0) {
    Write-Host "RESULT=FAIL ($($failures.Count) assertion(s))" -ForegroundColor Red
    $failures | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit 1
}
Write-Host 'RESULT=PASS'
exit 0
