#Requires -Version 5.1
<#
.SYNOPSIS
    Visual check for the Glass card top inner highlight leaking past the
    rounded top corners.

.DESCRIPTION
    GPUI clips a view's children with a rectangular ContentMask (no corner
    radius), so the 1 px full-width "top inner highlight" drawn by
    `GlassSurface` (apps/desktop-gpui/src/ui/glass.rs) is NOT cut by the
    surface's `.rounded(radius)` curve. The straight 1 px line therefore
    sticks out beyond the curve at the top-left and top-right corners.

    This script launches the desktop shell (read-only: it never edits the
    app), photographs its window into the screen bitmap, locates every wide
    glass card by its bright 1 px top line, measures how far that line is
    inset from the card's real left/right edges, and annotates the capture.

    Pure PowerShell 5.1 + .NET (System.Drawing, P/Invoke user32/gdi32):
    no computer use, no SendKeys and no external screenshot tool.

    Exit codes:
      0  every detected card has the highlight inset by ~Radius
      1  at least one card shows the leak (inset 0 or < Radius - 3)
      2  setup error (build failed, window never appeared, no card seen)

.EXAMPLE
    powershell -File tools\check-glass-corners.ps1 -SkipBuild

.EXAMPLE
    powershell -File tools\check-glass-corners.ps1 -Radius 10 -TimeoutSec 45
#>
[CmdletBinding()]
param(
    # Desktop shell binary to launch (built by this script unless -SkipBuild).
    [string]$ExePath = 'target\debug\xemnas.exe',

    # Data directory handed to the app through XEMNAS_DATA_DIR.
    [string]$DataDir = (Join-Path $env:TEMP 'xemnas-e2e2-20260928-183445\data'),

    # Documented radius.surface in logical pixels (96 DPI == physical px).
    [int]$Radius = 10,

    # Evidence directory; defaults to a timestamped folder under %TEMP%.
    [string]$OutDir = (Join-Path $env:TEMP ('xemnas\evidence\corners-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))),

    # Skip `cargo build` and use the binary already on disk.
    [switch]$SkipBuild,

    # Seconds to wait for the app's main window.
    [int]$TimeoutSec = 30,

    # Absolute luminance floor for a highlight pixel (see the measured values
    # documented next to GlassAnalyzer.IsHighlight). 55 keeps the real top
    # highlight (~64.5) while rejecting the card border (~45.3) and the rounded
    # corner antialias (~41-43).
    [int]$HighlightLum = 55,

    # Relative contrast a highlight pixel must beat against 4 px below.
    [int]$HighlightDelta = 10,

    # A top line must span at least this many pixels to count as a card.
    [int]$RowMin = 400,

    # Discard chosen card tops closer than this to the previous one.
    [int]$GroupSep = 20,

    # Cluster bright lines within this many rows (a card bottom border sits
    # ~14 px above the next card's top highlight); keep the brightest row.
    [int]$ClusterGap = 30,

    # A pixel differs from the sampled background by more than this in luminance.
    [int]$BgDelta = 8,

    # Ignore the outermost pixels of the window.
    [int]$Margin = 30
)

$ErrorActionPreference = 'Stop'

# Thresholds are the parameters above (defaults noted in the param block); the
# historical hardcoded values were HighlightDelta=10, RowMin=400, GroupSep=20,
# ClusterGap=30, BgDelta=8, Margin=30, HighlightLum=55.
function Write-Step([string]$Message) { Write-Host $Message }

# --- 1. build ---------------------------------------------------------------
if (-not $SkipBuild) {
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
    Write-Step 'BUILD cargo build -p desktop-gpui --bin xemnas'
    & cargo build -p desktop-gpui --bin xemnas
    if ($LASTEXITCODE -ne 0) {
        Write-Host "BUILD FAILED (exit $LASTEXITCODE)" -ForegroundColor Red
        exit 2
    }
}

if (-not (Test-Path -LiteralPath $ExePath)) {
    Write-Host "SETUP FAIL: binary not found at $ExePath (build it or drop -SkipBuild)" -ForegroundColor Red
    exit 2
}
$ExePath = (Resolve-Path -LiteralPath $ExePath).Path

if (-not (Test-Path -LiteralPath $DataDir)) {
    Write-Host "SETUP FAIL: DataDir not found: $DataDir" -ForegroundColor Red
    exit 2
}
$DataDir = (Resolve-Path -LiteralPath $DataDir).Path
$dbPath = Join-Path $DataDir 'state\app.db'
if (-not (Test-Path -LiteralPath $dbPath)) {
    Write-Host "SETUP FAIL: app.db not found at $dbPath" -ForegroundColor Red
    exit 2
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$rawPng   = Join-Path $OutDir 'window.png'
$annotPng = Join-Path $OutDir 'window-annotated.png'

# --- 2. native helpers + analyzer + SQLite row probe ------------------------
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -ReferencedAssemblies 'System.Drawing' -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;
using System.Text;

public struct XemnasRect { public int Left; public int Top; public int Right; public int Bottom; }

public static class XemnasWin {
    public delegate bool EnumWindowsProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumWindowsProc cb, IntPtr l);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out XemnasRect r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("kernel32.dll")] static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] static extern bool AttachThreadInput(uint a, uint b, bool f);
    [DllImport("user32.dll")] static extern bool SetProcessDPIAware();

    public static void MakeDpiAware() { SetProcessDPIAware(); }

    public static IntPtr FindWindow(uint pid, string titleContains) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((h, l) => {
            uint p;
            GetWindowThreadProcessId(h, out p);
            if (p != pid) return true;
            if (!IsWindowVisible(h)) return true;
            var sb = new StringBuilder(512);
            GetWindowText(h, sb, sb.Capacity);
            if (sb.ToString().IndexOf(titleContains, StringComparison.OrdinalIgnoreCase) >= 0) {
                found = h;
                return false;
            }
            return true;
        }, IntPtr.Zero);
        return found;
    }

    public static bool Activate(IntPtr hWnd) {
        if (IsIconic(hWnd)) ShowWindow(hWnd, 9); // SW_RESTORE
        uint fgPid;
        uint fg = GetWindowThreadProcessId(GetForegroundWindow(), out fgPid);
        uint me = GetCurrentThreadId();
        bool attached = fg != 0 && fg != me && AttachThreadInput(me, fg, true);
        bool ok = SetForegroundWindow(hWnd);
        BringWindowToTop(hWnd);
        ShowWindow(hWnd, 5); // SW_SHOW
        if (attached) AttachThreadInput(me, fg, false);
        return ok;
    }

    // Captures the given screen rectangle into a PNG and returns its packed
    // ARGB pixels (top-down, row-major).
    public static int[] CaptureToPng(string path, int l, int t, int r, int b, out int w, out int h) {
        w = r - l; h = b - t;
        if (w <= 0 || h <= 0) return null;
        using (var bmp = new Bitmap(w, h, PixelFormat.Format32bppArgb)) {
            using (var g = Graphics.FromImage(bmp)) {
                g.CopyFromScreen(l, t, 0, 0, new Size(w, h), CopyPixelOperation.SourceCopy);
            }
            bmp.Save(path, ImageFormat.Png);
            var data = bmp.LockBits(new Rectangle(0, 0, w, h), ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
            try {
                int[] px = new int[w * h];
                IntPtr p = data.Scan0;
                for (int y = 0; y < h; y++) {
                    Marshal.Copy(IntPtr.Add(p, y * data.Stride), px, y * w, w);
                }
                return px;
            } finally {
                bmp.UnlockBits(data);
            }
        }
    }
}

public class GlassCard {
    public int Index, Y0, Ca, Cb, Xa, Xb, InsetLeft, InsetRight;
    public bool Pass;
}

public class GlassReport {
    public int BgR, BgG, BgB, BgLum;
    public List<GlassCard> Cards = new List<GlassCard>();
}

public static class GlassAnalyzer {
    static double Lum(int argb) {
        int r = (argb >> 16) & 255, g = (argb >> 8) & 255, b = argb & 255;
        return 0.2126 * r + 0.7152 * g + 0.0722 * b;
    }
    // A highlight pixel is bright in absolute terms AND brighter than the
    // surface 4 px below it. The absolute floor is essential: the pure
    // relative test (lum(y) > lum(y+4)+10) also fires on the card border
    // (glass.border) and on the rounded-corner antialias, so it would report
    // the highlight starting at the border instead of after the radius.
    //
    // Measured luminances (96 DPI, Quiet Glass, RGB 0-255, 0.2126R+0.7152G+0.0722B):
    //   highlight (glass.border-top over fill) : ~64.5
    //   card border (glass.border)             : ~45.3
    //   rounded-corner antialias               : ~41-43
    //   glass fill / canvas                    : ~22 / ~15-17
    // Default floor 55 sits in the gap between the highlight (64.5) and the
    // border/antialias (<=45.3), so only real highlight pixels pass.
    static bool IsHighlight(int[] px, int w, int x, int y, int delta, int floor) {
        double a = Lum(px[y * w + x]);
        return a >= floor && a > Lum(px[(y + 4) * w + x]) + delta;
    }

    public static GlassReport Analyze(int[] px, int w, int h, int radius,
                                      int hlDelta, int hlFloor, int rowMin, int groupSep, int clusterGap, int bgDelta, int margin,
                                      int bottomLimit) {
        var rep = new GlassReport();

        // Background: most frequent color in the lower band of the window.
        var freq = new Dictionary<int, int>();
        int yStart = (int)(h * 0.60), yEnd = (int)(h * 0.95);
        for (int y = yStart; y < yEnd; y++) {
            int row = y * w;
            for (int x = 0; x < w; x++) {
                int c = px[row + x] & 0xFFFFFF;
                int v;
                freq.TryGetValue(c, out v);
                freq[c] = v + 1;
            }
        }
        int best = 0, bestCount = -1;
        foreach (var kv in freq) { if (kv.Value > bestCount) { best = kv.Key; bestCount = kv.Value; } }
        rep.BgR = (best >> 16) & 255; rep.BgG = (best >> 8) & 255; rep.BgB = best & 255;
        double bgLum = Lum(unchecked((int)0xFF000000) | best);
        rep.BgLum = (int)Math.Round(bgLum);

        // Candidate rows: bright thin line over a darker surface, spanning
        // most of the window width.
        var candidates = new List<int>();
        for (int y = 1; y < bottomLimit - 6; y++) {
            int count = 0;
            for (int x = margin; x < w - margin; x++) {
                if (IsHighlight(px, w, x, y, hlDelta, hlFloor)) count++;
            }
            if (count > rowMin) candidates.Add(y);
        }

        // Cluster candidate rows. A card's bottom border (glass.border) is also
        // a bright full-width line, but it is dimmer than the top highlight
        // (glass.border-top) and sits ~14 px above the next card's top line.
        // Cluster nearby rows and keep the brightest (average luminance) row of
        // each cluster, which is the top inner highlight we want to measure.
        var clusters = new List<List<int>>();
        foreach (var y in candidates) {
            if (clusters.Count == 0 ||
                y - clusters[clusters.Count - 1][clusters[clusters.Count - 1].Count - 1] > clusterGap) {
                clusters.Add(new List<int>());
            }
            clusters[clusters.Count - 1].Add(y);
        }
        var y0s = new List<int>();
        foreach (var cluster in clusters) {
            int bestRow = -1; double bestScore = -1;
            foreach (var y in cluster) {
                double sum = 0; int cnt = 0;
                for (int x = margin; x < w - margin; x++) {
                    if (IsHighlight(px, w, x, y, hlDelta, hlFloor)) { sum += Lum(px[y * w + x]); cnt++; }
                }
                if (cnt > 0) {
                    double avg = sum / cnt;
                    if (avg > bestScore) { bestScore = avg; bestRow = y; }
                }
            }
            if (bestRow >= 0) y0s.Add(bestRow);
        }
        // Defensive: drop chosen rows closer than groupSep to the previous one.
        var kept = new List<int>();
        foreach (var y in y0s) {
            if (kept.Count == 0 || y - kept[kept.Count - 1] >= groupSep) kept.Add(y);
        }
        y0s = kept;

        int idx = 0;
        foreach (var y0 in y0s) {
            int ymid = y0 + 25;
            // Never sample on the taskbar/footer that may cover the window bottom.
            if (ymid > bottomLimit - 3) ymid = bottomLimit - 3;
            if (ymid <= y0) continue;

            // Real horizontal extent of the card at ymid.
            int ca = -1, cb = -1;
            for (int x = margin; x < w - margin; x++) {
                if (Math.Abs(Lum(px[ymid * w + x]) - bgLum) > bgDelta) { ca = x; break; }
            }
            for (int x = w - margin - 1; x >= margin; x--) {
                if (Math.Abs(Lum(px[ymid * w + x]) - bgLum) > bgDelta) { cb = x; break; }
            }
            if (ca < 0 || cb < 0 || cb - ca < 200) continue;

            // Where the 1 px highlight actually starts/ends on the top row.
            int xa = -1, xb = -1;
            for (int x = ca; x <= cb; x++) { if (IsHighlight(px, w, x, y0, hlDelta, hlFloor)) { xa = x; break; } }
            for (int x = cb; x >= ca; x--) { if (IsHighlight(px, w, x, y0, hlDelta, hlFloor)) { xb = x; break; } }
            if (xa < 0 || xb < 0) continue;

            var card = new GlassCard();
            card.Index = idx++;
            card.Y0 = y0; card.Ca = ca; card.Cb = cb; card.Xa = xa; card.Xb = xb;
            card.InsetLeft = xa - ca;
            card.InsetRight = cb - xb;
            card.Pass = (card.InsetLeft >= radius - 3) && (card.InsetRight >= radius - 3);
            rep.Cards.Add(card);
        }
        return rep;
    }
}

// Minimal read-only SQLite reader: counts the leaf cells of one table.
// Enough for the tiny app DB; honors page size, interior pages and overflow-free
// records. Never writes to the database.
public static class SqliteProbe {
    static int Read16(byte[] b, int o) { return (b[o] << 8) | b[o + 1]; }
    static long PageStart(int p, int ps) { return (long)(p - 1) * ps; }

    static long Varint(byte[] b, ref int o) {
        long v = 0;
        for (int i = 0; i < 8; i++) {
            byte c = b[o++];
            v = (v << 7) | (uint)(c & 0x7F);
            if ((c & 0x80) == 0) return v;
        }
        v = (v << 8) | b[o++];
        return v;
    }

    static List<object> ParseRecord(byte[] b, int start, int payloadLen) {
        var vals = new List<object>();
        int o = start;
        int hdrLen = (int)Varint(b, ref o);
        int headerEnd = start + hdrLen;
        int body = headerEnd;
        while (o < headerEnd) {
            long st = Varint(b, ref o);
            if (st == 0) { vals.Add(null); }
            else if (st >= 1 && st <= 6) {
                int[] sizes = { 0, 1, 2, 3, 4, 6, 8 };
                int n = sizes[st];
                long v = 0;
                for (int i = 0; i < n; i++) v = (v << 8) | (uint)b[body++];
                vals.Add(v);
            } else if (st == 7) { vals.Add(0.0); body += 8; }
            else if (st == 8) { vals.Add(0L); }
            else if (st == 9) { vals.Add(1L); }
            else if (st >= 12) {
                int len = (int)((st - ((st % 2 == 0) ? 12 : 13)) / 2);
                if (st % 2 == 1) vals.Add(Encoding.UTF8.GetString(b, body, len));
                else vals.Add(null);
                body += len;
            }
        }
        return vals;
    }

    static uint FindRoot(byte[] db, int ps, string table) {
        var stack = new Stack<int>();
        stack.Push(1);
        while (stack.Count > 0) {
            int page = stack.Pop();
            int off = (int)PageStart(page, ps);
            int hdr = page == 1 ? 100 : 0;
            int type = db[off + hdr];
            if (type == 5) {
                int n = Read16(db, off + hdr + 3);
                int p = off + hdr + 12;
                for (int k = 0; k < n; k++) {
                    int cp = Read16(db, p); p += 2;
                    int co = off + cp;
                    int child = (db[co] << 24) | (db[co + 1] << 16) | (db[co + 2] << 8) | db[co + 3];
                    stack.Push(child);
                }
                int right = (db[off + hdr + 8] << 24) | (db[off + hdr + 9] << 16) | (db[off + hdr + 10] << 8) | db[off + hdr + 11];
                stack.Push(right);
            } else if (type == 13) {
                int n = Read16(db, off + hdr + 3);
                int p = off + hdr + 8;
                for (int k = 0; k < n; k++) {
                    int cp = Read16(db, p); p += 2;
                    int co = off + cp;
                    int o = co;
                    long payload = Varint(db, ref o);
                    Varint(db, ref o); // rowid
                    var vals = ParseRecord(db, o, (int)payload);
                    if (vals.Count >= 4 && vals[0] is string && (string)vals[0] == "table" &&
                        vals[1] is string && (string)vals[1] == table) {
                        return (uint)(long)vals[3];
                    }
                }
            }
        }
        return 0;
    }

    static int CountTable(byte[] db, int ps, int root) {
        int count = 0;
        var stack = new Stack<int>();
        stack.Push(root);
        while (stack.Count > 0) {
            int page = stack.Pop();
            int off = (int)PageStart(page, ps);
            int hdr = page == 1 ? 100 : 0;
            int type = db[off + hdr];
            if (type == 13) {
                count += Read16(db, off + hdr + 3);
            } else if (type == 5) {
                int n = Read16(db, off + hdr + 3);
                int p = off + hdr + 12;
                for (int k = 0; k < n; k++) {
                    int cp = Read16(db, p); p += 2;
                    int co = off + cp;
                    int child = (db[co] << 24) | (db[co + 1] << 16) | (db[co + 2] << 8) | db[co + 3];
                    stack.Push(child);
                }
                int right = (db[off + hdr + 8] << 24) | (db[off + hdr + 9] << 16) | (db[off + hdr + 10] << 8) | db[off + hdr + 11];
                stack.Push(right);
            }
        }
        return count;
    }

    public static int CountRows(string path, string table) {
        byte[] db = System.IO.File.ReadAllBytes(path);
        int ps = Read16(db, 16);
        if (ps == 1) ps = 65536;
        uint root = FindRoot(db, ps, table);
        if (root == 0) return -1;
        return CountTable(db, ps, (int)root);
    }
}
'@

# --- 3. report the project count baked into the DataDir ---------------------
$projectCount = -1
try { $projectCount = [SqliteProbe]::CountRows($dbPath, 'projects') } catch { $projectCount = -1 }
Write-Host "PROJECTS_DB=$projectCount data=$DataDir"

# --- 4. launch the shell ----------------------------------------------------
[XemnasWin]::MakeDpiAware()
$env:XEMNAS_DATA_DIR = $DataDir
Write-Host "LAUNCH exe=$ExePath"
$proc = Start-Process -FilePath $ExePath -PassThru

$exitCode = 2
try {
    # Wait for the main window (visible, owned by our PID, title contains xemnas).
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    $hwnd = [IntPtr]::Zero
    while ((Get-Date) -lt $deadline) {
        if ($proc.HasExited) {
            Write-Host "SETUP FAIL: process exited early (code $($proc.ExitCode))" -ForegroundColor Red
            exit 2
        }
        $hwnd = [XemnasWin]::FindWindow([uint32]$proc.Id, 'xemnas')
        if ($hwnd -ne [IntPtr]::Zero) { break }
        Start-Sleep -Milliseconds 100
    }
    if ($hwnd -eq [IntPtr]::Zero) {
        Write-Host "SETUP FAIL: no xemnas window within ${TimeoutSec}s" -ForegroundColor Red
        exit 2
    }

    [void][XemnasWin]::Activate($hwnd)
    Start-Sleep -Milliseconds 900

    $rect = New-Object XemnasRect
    if (-not [XemnasWin]::GetWindowRect($hwnd, [ref]$rect)) {
        Write-Host 'SETUP FAIL: GetWindowRect failed' -ForegroundColor Red
        exit 2
    }
    Write-Host "WINDOW rect=$($rect.Left),$($rect.Top)..$($rect.Right),$($rect.Bottom)"

    $w = 0; $h = 0
    $px = [XemnasWin]::CaptureToPng($rawPng, $rect.Left, $rect.Top, $rect.Right, $rect.Bottom, [ref]$w, [ref]$h)
    if ($null -eq $px -or $w -le 0 -or $h -le 0) {
        Write-Host 'SETUP FAIL: screen capture returned no pixels' -ForegroundColor Red
        exit 2
    }
    Write-Host "CAPTURE ${w}x${h} saved=$rawPng"

    # The window may extend past the monitor work area (taskbar/footer), so
    # rows below this limit are not app content and must not be measured.
    $wa = [System.Windows.Forms.Screen]::FromHandle($hwnd).WorkingArea
    $bottomLimit = [Math]::Min($h, $wa.Bottom - $rect.Top)
    Write-Host "WORKAREA bottom=$($wa.Bottom) bottomLimit=$bottomLimit"

    $report = [GlassAnalyzer]::Analyze($px, $w, $h, $Radius, $HighlightDelta, $HighlightLum, $RowMin, $GroupSep, $ClusterGap, $BgDelta, $Margin, $bottomLimit)

    Write-Host ("BACKGROUND rgb({0},{1},{2}) lum={3}" -f $report.BgR, $report.BgG, $report.BgB, $report.BgLum)
    Write-Host ("THRESHOLDS highlight_lum={0} highlight_delta={1} row_min={2} group_sep={3} cluster_gap={4} bg_delta={5} edge_margin={6} radius={7}" -f `
        $HighlightLum, $HighlightDelta, $RowMin, $GroupSep, $ClusterGap, $BgDelta, $Margin, $Radius)

    $fmt = '{0,3} {1,5} {2,5} {3,5} {4,5} {5,5} {6,9} {7,9} {8}'
    Write-Host ($fmt -f 'idx', 'y0', 'ca', 'cb', 'xa', 'xb', 'inset_esq', 'inset_dir', 'verdict')
    foreach ($c in $report.Cards) {
        $verdict = if ($c.Pass) { 'PASS' } else { 'FAIL' }
        Write-Host ($fmt -f $c.Index, $c.Y0, $c.Ca, $c.Cb, $c.Xa, $c.Xb, $c.InsetLeft, $c.InsetRight, $verdict)
    }

    # --- 5. annotate the capture --------------------------------------------
    $bmp = [System.Drawing.Bitmap]::FromFile($rawPng)
    try {
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $penExp = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(255, 255, 64, 64), 1)
        $penHl  = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(255, 64, 255, 64), 2)
        $penEnd = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(255, 64, 192, 255), 2)
        try {
            foreach ($c in $report.Cards) {
                # Expected left corner zone where the highlight should begin.
                $g.DrawRectangle($penExp, $c.Ca, $c.Y0, $Radius, [Math]::Max($Radius, 8))
                # Where the highlight really starts / ends.
                $g.DrawLine($penHl, $c.Xa, $c.Y0 - 4, $c.Xa, $c.Y0 + 8)
                $g.DrawLine($penEnd, $c.Xb, $c.Y0 - 4, $c.Xb, $c.Y0 + 8)
            }
        } finally {
            $penExp.Dispose(); $penHl.Dispose(); $penEnd.Dispose(); $g.Dispose()
        }
        $bmp.Save($annotPng, [System.Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $bmp.Dispose()
    }
    Write-Host "ANNOTATED saved=$annotPng"

    if ($report.Cards.Count -eq 0) {
        Write-Host 'SETUP FAIL: 0 glass cards detected (the script cannot see the screen)' -ForegroundColor Red
        $exitCode = 2
    } else {
        $fail = @($report.Cards | Where-Object { -not $_.Pass }).Count
        $pass = $report.Cards.Count - $fail
        Write-Host "RESULT cards=$($report.Cards.Count) pass=$pass fail=$fail"
        $exitCode = if ($fail -gt 0) { 1 } else { 0 }
    }
} catch {
    Write-Host "ERROR: $($_.Exception.Message)" -ForegroundColor Red
    $exitCode = 2
} finally {
    if ($null -ne $proc) {
        try {
            if (-not $proc.HasExited) {
                Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
                $proc.WaitForExit()
            }
        } catch { }
    }
}

if ($exitCode -eq 0) { Write-Host 'RESULT=PASS' -ForegroundColor Green }
elseif ($exitCode -eq 1) { Write-Host 'RESULT=FAIL' -ForegroundColor Red }
else { Write-Host 'RESULT=SETUP-ERROR' -ForegroundColor Red }
exit $exitCode
