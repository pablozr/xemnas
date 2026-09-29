<#
.SYNOPSIS
    End-to-end evidence for ticket 11: outbox import, diagnostic rejection and
    deduplication.

.DESCRIPTION
    Uses one temporary XEMNAS_DATA_DIR and the real adapter CLI plus the real
    desktop binary:
      Phase A (desktop closed): the adapter cannot reach the API, so it writes
        the envelope to outbox/pending; the secret in the fixture must be
        redacted and no receipt exists yet.
      Phase B (desktop opens): boot drain accepts the adapter item and rejects a
        planted corrupt item with a safe diagnostic; pending never keeps a
        finished item and the secret never reaches disk.
      Phase C (desktop running): the adapter posts the same capture again; the
        API deduplicates it and the database is unchanged.

    Writes RESULT=PASS/FAIL and exits 0/1. Pure ASCII on purpose.

.EXAMPLE
    powershell -NoProfile -File tests\e2e\capture-outbox.ps1
#>
[CmdletBinding()]
param(
    [string]$ExePath
)

$ErrorActionPreference = 'Stop'

# cargo lives under the user profile in this environment.
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
if (-not $ExePath) {
    $ExePath = Join-Path $repoRoot 'target\debug\xemnas.exe'
}

$failures = New-Object System.Collections.Generic.List[string]

function Assert([bool]$Condition, [string]$Message) {
    if ($Condition) { Write-Host "  ok   $Message" }
    else { $failures.Add($Message); Write-Host "  FAIL $Message" -ForegroundColor Red }
}

function Get-Sha256Hex([string]$Value) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($Value)
        return (($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString('x2') }) -join '')
    } finally {
        $sha.Dispose()
    }
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

# --- prerequisites ---------------------------------------------------------

$python = Get-Command python -ErrorAction SilentlyContinue
if (-not $python) { throw 'python is required for the outbox E2E seed/query' }
$node = Get-Command node -ErrorAction SilentlyContinue
if (-not $node) { throw 'node is required to run the adapter CLI' }
$npm = Get-Command npm -ErrorAction SilentlyContinue
if (-not $npm) { throw 'npm is required to build the adapter CLI' }

$adapterDir = Join-Path $repoRoot 'adapters\opencode'
$cliPath = Join-Path $adapterDir 'dist\src\cli\send-fixture.js'
$fixtureSource = Join-Path $adapterDir 'tests\fixtures\e2e\session.json'
if (-not (Test-Path -LiteralPath $fixtureSource)) {
    throw "adapter fixture not found: $fixtureSource"
}

Write-Host "OUTBOX-E2E building adapter"
if (-not (Test-Path -LiteralPath (Join-Path $adapterDir 'node_modules'))) {
    Push-Location $adapterDir
    try { $code = Invoke-Native { & npm ci } } finally { Pop-Location }
    if ($code -ne 0) { throw "npm ci failed with exit $code" }
}
Push-Location $adapterDir
try { $code = Invoke-Native { & npm run build } } finally { Pop-Location }
if ($code -ne 0) { throw "npm run build failed with exit $code" }
if (-not (Test-Path -LiteralPath $cliPath)) {
    throw "adapter CLI was not built at $cliPath"
}

Write-Host "OUTBOX-E2E building desktop"
$code = Invoke-Native { & cargo build -p desktop-gpui --locked }
if ($code -ne 0) { throw "cargo build -p desktop-gpui failed with exit $code" }
$ExePath = (Resolve-Path -LiteralPath $ExePath).Path

# --- temporary workspace ---------------------------------------------------

$dataDir = Join-Path $env:TEMP ("xemnas-capture-outbox-" + [Guid]::NewGuid().ToString('N'))
$stateDir = Join-Path $dataDir 'state'
$projDir = Join-Path $dataDir 'project'
$outboxDir = Join-Path $dataDir 'outbox'
$fixtureDir = Join-Path $dataDir 'fixture'
$adapterState = Join-Path $dataDir 'adapter'
New-Item -ItemType Directory -Force -Path $stateDir, $projDir, $outboxDir, $fixtureDir, $adapterState | Out-Null
$dbPath = Join-Path $stateDir 'app.db'

$env:XEMNAS_DATA_DIR = $dataDir
$env:XEMNAS_OUTBOX_DIR = $outboxDir
$env:XEMNAS_OUTBOX_ACCEPTED_RETENTION_DAYS = '7'

Write-Host "OUTBOX-E2E data_dir=$dataDir"

# Reads the database into a JSON summary; missing tables count as zero.
$queryProgram = @'
import sqlite3, json, sys
db = sys.argv[1]
con = sqlite3.connect(db)
def has(name):
    return con.execute(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
        (name,),
    ).fetchone()[0] > 0
def count(name):
    return con.execute("SELECT COUNT(*) FROM " + name).fetchone()[0] if has(name) else 0
out = {
    "receipts": count("capture_receipts"),
    "artifacts": count("capture_artifacts"),
    "checkpoints": count("adapter_checkpoints"),
    "capture_id": None,
    "artifact_has_secret": False,
}
if has("capture_receipts"):
    row = con.execute("SELECT capture_id FROM capture_receipts LIMIT 1").fetchone()
    out["capture_id"] = row[0] if row else None
if has("capture_artifacts"):
    secret = con.execute(
        "SELECT COUNT(*) FROM capture_artifacts WHERE content LIKE '%sk-test-%'"
    ).fetchone()[0]
    out["artifact_has_secret"] = secret > 0
con.close()
print(json.dumps(out))
'@
$queryPath = Join-Path $dataDir 'query.py'
[IO.File]::WriteAllText($queryPath, $queryProgram, [Text.UTF8Encoding]::new($false))

function Get-DatabaseState {
    $raw = (& $python.Source $queryPath $dbPath | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) { throw "database query failed with exit $LASTEXITCODE" }
    return ($raw | ConvertFrom-Json)
}

# --- seed: register the project exactly as the Rust allow-list expects -----

$rewriteProgram = @'
import json, os, sys
src = sys.argv[1]
dst = sys.argv[2]
project_dir = sys.argv[3]
canonical = os.path.realpath(project_dir)
if canonical.startswith("\\\\?\\"):
    canonical = canonical[4:]
with open(src, "r", encoding="utf-8") as handle:
    document = json.load(handle)
document["directory"] = canonical
with open(dst, "w", encoding="utf-8", newline="") as handle:
    json.dump(document, handle, ensure_ascii=True, indent=2)
    handle.write("\n")
print(canonical)
'@
$rewritePath = Join-Path $dataDir 'rewrite_fixture.py'
[IO.File]::WriteAllText($rewritePath, $rewriteProgram, [Text.UTF8Encoding]::new($false))

$canonicalLocation = (& $python.Source $rewritePath $fixtureSource (Join-Path $fixtureDir 'session.json') $projDir | Out-String).Trim()
if ($LASTEXITCODE -ne 0) { throw "fixture rewrite failed with exit $LASTEXITCODE" }
Assert (-not [string]::IsNullOrEmpty($canonicalLocation)) 'the fixture project path was canonicalized'
Assert (Test-Path -LiteralPath (Join-Path $fixtureDir 'session.json')) 'the rewritten fixture exists'

$seedProgram = @'
import sqlite3, sys
db = sys.argv[1]
location = sys.argv[2]
con = sqlite3.connect(db)
con.executescript("""
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    location TEXT UNIQUE NOT NULL,
    registered_at TEXT NOT NULL
);
""")
con.execute(
    "INSERT INTO projects (id, location, registered_at) VALUES (?, ?, ?)",
    ("seed-project", location, "2026-01-01T00:00:00Z"),
)
con.commit()
con.close()
'@
$seedPath = Join-Path $dataDir 'seed.py'
[IO.File]::WriteAllText($seedPath, $seedProgram, [Text.UTF8Encoding]::new($false))
& $python.Source $seedPath $dbPath $canonicalLocation
if ($LASTEXITCODE -ne 0) { throw "seed failed with exit $LASTEXITCODE" }

# --- Phase A: desktop closed -> the adapter writes to the outbox -----------

Write-Host "OUTBOX-E2E phase A (desktop closed)"
$cliStdoutA = Join-Path $dataDir 'cli-a.stdout'
$cliStderr = Join-Path $dataDir 'cli-a.stderr'
$code = Invoke-Native { & node $cliPath --fixture $fixtureDir --state-dir $adapterState --outbox-dir $outboxDir --data-dir $dataDir 1> $cliStdoutA 2> $cliStderr }
if ($code -ne 0) {
    $detail = (Get-Content -LiteralPath $cliStderr -Raw -ErrorAction SilentlyContinue)
    throw "adapter CLI (phase A) failed with exit ${code}: $detail"
}
$summaryA = ((Get-Content -LiteralPath $cliStdoutA -Raw).Trim() | ConvertFrom-Json)
Assert ($summaryA.outcome -eq 'outbox') "phase A wrote to the outbox (outcome=$($summaryA.outcome))"
Assert (-not [string]::IsNullOrEmpty($summaryA.idempotency_key)) 'phase A reported an idempotency key'
Assert (-not [string]::IsNullOrEmpty($summaryA.capture_id)) 'phase A reported a capture id'

$expectedName = (Get-Sha256Hex $summaryA.idempotency_key) + '.json'
$pendingFile = Join-Path $outboxDir ('pending\' + $expectedName)
Assert (Test-Path -LiteralPath $pendingFile) 'the pending file is named sha256(idempotency_key).json'
if (-not [string]::IsNullOrEmpty($summaryA.outbox_path)) {
    Assert ((Split-Path -Leaf $summaryA.outbox_path) -eq $expectedName) 'the CLI outbox_path matches the expected file name'
}
$pendingItems = @(Get-ChildItem -LiteralPath (Join-Path $outboxDir 'pending') -Filter '*.json' -File -ErrorAction SilentlyContinue)
Assert ($pendingItems.Count -eq 1) 'exactly one pending item was written'
Assert ((Get-Content -LiteralPath $pendingFile -Raw) -notmatch 'sk-test-') 'the outbox envelope was redacted (no secret)'

$stateA = Get-DatabaseState
Assert ($stateA.receipts -eq 0) 'no receipt exists while the desktop is closed'

# --- plant a diagnosable rejection -----------------------------------------

$plantedName = 'zz-corrupt.json'
$plantedPath = Join-Path $outboxDir ('pending\' + $plantedName)
[IO.File]::WriteAllText($plantedPath, '{ "broken": "sk-test-PLANTED" ', [Text.UTF8Encoding]::new($false))

# --- Phase B: desktop opens -> drain ---------------------------------------

Write-Host "OUTBOX-E2E phase B (desktop opens)"
$proc = Start-Process -FilePath $ExePath -PassThru

try {
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
    Start-Sleep -Seconds 3
    $proc.Refresh()
    Assert (-not $proc.HasExited) 'the app was still running while the outbox drained'

    $pendingAfter = @(Get-ChildItem -LiteralPath (Join-Path $outboxDir 'pending') -Filter '*.json' -File -ErrorAction SilentlyContinue)
    Assert ($pendingAfter.Count -eq 0) 'no finished item is left in pending'
    Assert (Test-Path -LiteralPath (Join-Path $outboxDir ('accepted\' + $expectedName))) 'the adapter capture was accepted'
    Assert (Test-Path -LiteralPath (Join-Path $outboxDir ('rejected\' + $plantedName))) 'the corrupt item was rejected'

    $rejectedText = Get-Content -LiteralPath (Join-Path $outboxDir ('rejected\' + $plantedName)) -Raw
    $rejected = $rejectedText | ConvertFrom-Json
    Assert ($rejected.code -eq 'invalid_json') "the rejection records invalid_json (code=$($rejected.code))"
    Assert ($rejectedText -notmatch 'sk-test-PLANTED') 'the rejection diagnostic carries no envelope content'
    $rejectedAll = (Get-ChildItem -LiteralPath (Join-Path $outboxDir 'rejected') -File | ForEach-Object { Get-Content -LiteralPath $_.FullName -Raw }) -join "`n"
    Assert ($rejectedAll -notmatch 'sk-test-PLANTED') 'no rejected file carries the planted secret'

    $stateB = Get-DatabaseState
    Assert ($stateB.receipts -eq 1) 'exactly one receipt was imported'
    Assert ($stateB.artifacts -ge 1) 'at least one artifact was persisted'
    Assert (-not $stateB.artifact_has_secret) 'no persisted artifact carries the secret'
    Assert ($stateB.capture_id -eq $summaryA.capture_id) 'the receipt capture id matches the CLI capture id'
    Assert ($stateB.checkpoints -eq 1) 'the adapter checkpoint was written for the accepted capture'

    # --- Phase C: desktop running -> direct POST deduplicates ---------------

    Write-Host "OUTBOX-E2E phase C (desktop running, dedup)"
    $discoveryPath = Join-Path $stateDir 'discovery.json'
    Assert (Test-Path -LiteralPath $discoveryPath) 'the running app wrote its discovery file'
    Assert (Test-Path -LiteralPath (Join-Path $stateDir 'api-token')) 'the running app wrote its api token'

    $cliStdoutC = Join-Path $dataDir 'cli-c.stdout'
    $cliStderrC = Join-Path $dataDir 'cli-c.stderr'
    $code = Invoke-Native { & node $cliPath --fixture $fixtureDir --discovery $discoveryPath --state-dir $adapterState --outbox-dir $outboxDir --data-dir $dataDir 1> $cliStdoutC 2> $cliStderrC }
    if ($code -ne 0) {
        $detail = (Get-Content -LiteralPath $cliStderrC -Raw -ErrorAction SilentlyContinue)
        throw "adapter CLI (phase C) failed with exit ${code}: $detail"
    }
    $summaryC = ((Get-Content -LiteralPath $cliStdoutC -Raw).Trim() | ConvertFrom-Json)
    Assert ($summaryC.outcome -eq 'sent') "phase C posted directly (outcome=$($summaryC.outcome))"
    Assert ($summaryC.capture_id -eq $summaryA.capture_id) 'phase C replayed the same capture id'

    $stateC = Get-DatabaseState
    Assert ($stateC.receipts -eq 1) 'replay did not add a receipt'
    Assert ($stateC.artifacts -eq $stateB.artifacts) 'replay did not add artifacts'
    Assert ($stateC.checkpoints -eq 1) 'replay did not add a checkpoint'
} finally {
    # Never leave the app, its API or the session files behind, even when a
    # phase throws: a leaked process would contaminate the next run.
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    try { $proc.WaitForExit() } catch { }
    Remove-Item -LiteralPath (Join-Path $stateDir 'discovery.json'), (Join-Path $stateDir 'api-token') -Force -ErrorAction SilentlyContinue
}

# --- report ----------------------------------------------------------------

if ($failures.Count -gt 0) {
    Write-Host "OUTBOX-E2E data_dir kept at $dataDir" -ForegroundColor Red
    Write-Host "RESULT=FAIL ($($failures.Count) assertion(s))" -ForegroundColor Red
    $failures | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit 1
}

Remove-Item -LiteralPath $dataDir -Recurse -Force -ErrorAction SilentlyContinue
Write-Host 'RESULT=PASS'
exit 0
