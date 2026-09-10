<#
.SYNOPSIS
  Guided capture: one gesture at a time, so every key event is labelled with the
  gesture that produced it.

.DESCRIPTION
  For each gesture in the list the script asks you to perform it, then records
  every F13-F24 and modifier event from the first key it sees until -Seconds
  (default 3) after that first key. Nothing else is recorded, so a run of eight
  events under one prompt is the firmware, not you repeating yourself.

  The gesture list defaults to every input in your Create Companion config
  (%APPDATA%\CreateCompanion\config.toml) with the key it is supposed to send,
  so each capture also says MATCH or MISMATCH. Override with -Gestures.

  A plan file (-PlanFile) has one prompt per line; a line may end in "expect=<key>"
  (e.g. "tap (2 fingers) expect=Shift+F14") to get the same MATCH / MISMATCH check.
  Plans for the Tune and the Touch live in tools/plans/.

  Output: a log with one "### gesture: <name>" header per gesture, ready for
  tools/keymon_analyze.py, and a summary table: presses per gesture, ONCE or
  STREAM. The engine keeps running; this hook passes keys on. Run in your own
  PowerShell window from the repository root.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File tools\gesture_capture.ps1
  powershell -ExecutionPolicy Bypass -File tools\gesture_capture.ps1 -Gestures "2-finger swipe up","3-finger tap" -Seconds 4
  powershell -ExecutionPolicy Bypass -File tools\gesture_capture.ps1 -Only TUNE_CCW,TUNE_SWIPE_LEFT_2F -Append
  powershell -ExecutionPolicy Bypass -File tools\gesture_capture.ps1 -PlanFile tools\plans\burst-tune.txt -Seconds 4 -Out tools\plans\burst-tune.log
  powershell -ExecutionPolicy Bypass -File tools\gesture_capture.ps1 -PlanFile tools\plans\census-touch.txt -Only "Round 1" -Seconds 4 -Out tools\plans\census-touch.log
  python tools\keymon_analyze.py capture.log
#>
param(
  [string[]]$Gestures,
  [string]$PlanFile, # text file, one gesture instruction per line (# comments and blank lines ignored)
  [int]$Seconds = 3,
  [string]$Out,      # default: capture.log in the repository root
  [string]$Config = "$env:APPDATA\CreateCompanion\config.toml",
  [string[]]$Only,   # event ids or name fragments to test, e.g. TUNE_CCW,"swipe down (1"
  [switch]$Append,   # add to the log instead of starting a new one
  [switch]$DryRun    # print the gesture plan and exit
)
$repoRoot = Split-Path $PSScriptRoot -Parent
if (-not $Out) { $Out = Join-Path $repoRoot "capture.log" }
$analyzer = Join-Path $PSScriptRoot "keymon_analyze.py"

Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class GCap {
  public delegate IntPtr HookProc(int code, IntPtr wParam, IntPtr lParam);
  [StructLayout(LayoutKind.Sequential)]
  public struct KBDLLHOOKSTRUCT { public uint vkCode; public uint scanCode; public uint flags; public uint time; public UIntPtr dwExtraInfo; }
  [StructLayout(LayoutKind.Sequential)]
  public struct MSG { public IntPtr hwnd; public uint message; public UIntPtr wParam; public IntPtr lParam; public uint time; public int x; public int y; }
  [DllImport("user32.dll", SetLastError = true)] public static extern IntPtr SetWindowsHookEx(int idHook, HookProc lpfn, IntPtr hMod, uint dwThreadId);
  [DllImport("user32.dll")] public static extern bool UnhookWindowsHookEx(IntPtr hhk);
  [DllImport("user32.dll")] public static extern IntPtr CallNextHookEx(IntPtr hhk, int nCode, IntPtr wParam, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool PeekMessage(out MSG msg, IntPtr hWnd, uint min, uint max, uint remove);
  [DllImport("kernel32.dll")] public static extern IntPtr GetModuleHandle(string name);

  public static HookProc Proc;
  public static IntPtr Hook;
  public static List<string> Lines = new List<string>();
  public static long FirstF = 0;          // ms tick of the first F-key press in this capture, 0 = none yet
  public static bool Recording = false;
  public static List<string> Modifiers = new List<string>(); // modifiers seen down at the moment of each F press

  static readonly Dictionary<int, bool> modDown = new Dictionary<int, bool>();
  static string Name(uint vk) {
    if (vk >= 0x7C && vk <= 0x87) return "F" + (vk - 0x70 + 1);
    switch (vk) {
      case 0xA0: return "LShift"; case 0xA1: return "RShift"; case 0xA2: return "LCtrl"; case 0xA3: return "RCtrl";
      case 0xA4: return "LAlt"; case 0xA5: return "RAlt"; case 0x5B: return "LWin"; case 0x5C: return "RWin";
      case 0x10: return "Shift"; case 0x11: return "Ctrl"; case 0x12: return "Alt";
    }
    return null;
  }
  static string HeldMods() {
    var s = "";
    foreach (var kv in modDown) if (kv.Value) s += (s.Length > 0 ? "+" : "") + Name((uint)kv.Key).TrimStart('L', 'R');
    return s.Length == 0 ? "-" : s;
  }
  public static void Reset() { Lines.Clear(); FirstF = 0; Recording = true; }
  public static void Stop() { Recording = false; }
  public static IntPtr Callback(int code, IntPtr wParam, IntPtr lParam) {
    if (code == 0) {
      var k = (KBDLLHOOKSTRUCT)Marshal.PtrToStructure(lParam, typeof(KBDLLHOOKSTRUCT));
      bool up = (k.flags & 0x80) != 0;
      bool injected = (k.flags & 0x10) != 0;
      string name = Name(k.vkCode);
      bool isF = k.vkCode >= 0x7C && k.vkCode <= 0x87;
      if (name != null && !isF && !injected) modDown[(int)k.vkCode] = !up;
      if (Recording && name != null && !injected) {
        if (isF && !up && FirstF == 0) FirstF = Environment.TickCount;
        long lag = (long)((uint)Environment.TickCount - k.time);
        Lines.Add(string.Format("{0:HH:mm:ss.fff}  hw+{1,4}ms  {2,-4} {3,-8} vk=0x{4:X2} sc=0x{5:X3} physical mods-at-this-instant={6}",
          DateTime.Now, lag, up ? "UP" : "DOWN", name, k.vkCode, k.scanCode, HeldMods()));
      }
    }
    return CallNextHookEx(Hook, code, wParam, lParam);
  }
  public static void Install() {
    Proc = new HookProc(Callback);
    Hook = SetWindowsHookEx(13, Proc, GetModuleHandle(null), 0);
    if (Hook == IntPtr.Zero) throw new Exception("SetWindowsHookEx failed: " + Marshal.GetLastWin32Error());
  }
  public static void Pump() { MSG m; while (PeekMessage(out m, IntPtr.Zero, 0, 0, 1)) { } }
}
'@

# ---- gesture list: from the config unless given ----
$plan = @()
if ($PlanFile) {
  if (-not (Test-Path $PlanFile)) { Write-Host "Plan file not found: $PlanFile" -ForegroundColor Red; exit 1 }
  foreach ($line in Get-Content $PlanFile) {
    $t = $line.Trim()
    if (-not $t -or $t.StartsWith("#")) { continue }
    $expect = $null
    if ($t -match '^(.*?)\s+expect=(\S+)$') { $t = $Matches[1]; $expect = $Matches[2] }
    $plan += [pscustomobject]@{ Name = $t; Expect = $expect; Event = $null }
  }
} elseif ($Gestures) {
  # Each -Gestures element is one prompt, commas included (from a PowerShell prompt, "a","b" arrives as two elements).
  foreach ($g in @($Gestures | Where-Object { $_.Trim() })) { $plan += [pscustomobject]@{ Name = $g.Trim(); Expect = $null; Event = $null } }
} elseif (Test-Path $Config) {
  $cur = $null
  foreach ($line in Get-Content $Config) {
    if ($line -match '^\[transport\.([A-Z0-9_]+)\]') { $cur = [pscustomobject]@{ Event = $Matches[1]; Key = $null; Mods = "none" }; $plan += $cur; continue }
    if ($cur -and $line -match '^key\s*=\s*"([^"]+)"') { $cur.Key = $Matches[1]; continue }
    if ($cur -and $line -match '^mods\s*=\s*"([^"]+)"') { $cur.Mods = $Matches[1]; continue }
    if ($line -match '^\[' ) { $cur = $null }
  }
  $plan = $plan | Where-Object { $_.Key } | ForEach-Object {
    $mods = if ($_.Mods -eq "none") { "" } else { (($_.Mods -split '\+') | ForEach-Object { $_.Substring(0,1).ToUpper() + $_.Substring(1) }) -join "+" }
    $expect = if ($mods) { "$mods+$($_.Key)" } else { $_.Key }
    # TUNE_SWIPE_DOWN_3F -> "Tune swipe down (3 fingers)"
    $parts = $_.Event -split '_'
    $module = if ($parts[0] -eq "TUNE") { "Tune" } elseif ($parts[0] -eq "LEFT") { "Left Touch" } else { "Right Touch" }
    $rest = if ($parts[0] -eq "TUNE") { $parts[1..($parts.Length-1)] } else { $parts[2..($parts.Length-1)] }
    $fingers = ""
    if ($rest[-1] -match '^([1-4])F$') { $fingers = " ($($Matches[1]) finger$(if ($Matches[1] -ne '1') {'s'}))"; $rest = $rest[0..($rest.Length-2)] }
    $gesture = (($rest -join ' ').ToLower()) -replace '^cw$','dial clockwise' -replace '^ccw$','dial counter-clockwise'
    [pscustomobject]@{ Name = "$module $gesture$fingers"; Expect = $expect; Event = $_.Event }
  }
} else {
  Write-Host "No config at $Config and no -Gestures given." -ForegroundColor Red; exit 1
}

if ($Only) {
  $Only = @($Only | ForEach-Object { $_ -split "," } | Where-Object { $_ })  # -File passes "a,b" as one string
  $plan = @($plan | Where-Object { $g = $_; ($Only | Where-Object { $g.Event -eq $_ -or $g.Name -like "*$_*" }).Count -gt 0 })
  if ($plan.Count -eq 0) { Write-Host "Nothing matches -Only $($Only -join ', ')" -ForegroundColor Red; exit 1 }
}

if ($DryRun) {
  Write-Host "Plan ($($plan.Count) gestures):"
  $plan | ForEach-Object { Write-Host ("  {0,-42} {1}" -f $_.Name, $_.Expect) }
  exit 0
}

[GCap]::Install()
$stamp = "# gesture capture $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')  post-first-key window ${Seconds}s"
if ($Append) { $stamp | Out-File $Out -Append -Encoding utf8 } else { $stamp | Out-File $Out -Encoding utf8 }
Write-Host "Guided capture: $($plan.Count) gestures, $Seconds s after the first key each. Log: $Out" -ForegroundColor Cyan
Write-Host "At each prompt: press Enter, then perform the gesture ONCE. Press S to skip a gesture, Q to quit." -ForegroundColor DarkGray
Write-Host ""

function WaitKey {
  while (-not [Console]::KeyAvailable) { [GCap]::Pump(); Start-Sleep -Milliseconds 10 }
  return [Console]::ReadKey($true)
}

$results = @()
try {
  $n = 0
  foreach ($g in $plan) {
    $n++
    $exp = if ($g.Expect) { "  (expect $($g.Expect))" } else { "" }
    Write-Host ("[{0}/{1}] Next: {2}{3}" -f $n, $plan.Count, $g.Name, $exp) -ForegroundColor Yellow
    Write-Host "      Enter = ready, S = skip, Q = quit" -ForegroundColor DarkGray
    $k = WaitKey
    if ($k.Key -eq "Q") { break }
    if ($k.Key -eq "S") {
      "### gesture: $($g.Name) [skipped]" | Out-File $Out -Append -Encoding utf8
      $results += [pscustomobject]@{ Gesture = $g.Name; Presses = "-"; SpanMs = "-"; Keys = ""; Verdict = "skipped" }
      continue
    }
    [GCap]::Reset()
    Write-Host "      Go: perform the gesture once..." -ForegroundColor Green
    # wait for the first F-key, then keep recording for $Seconds after it
    $deadline = (Get-Date).AddSeconds(30)
    while ([GCap]::FirstF -eq 0 -and (Get-Date) -lt $deadline) { [GCap]::Pump(); Start-Sleep -Milliseconds 5 }
    if ([GCap]::FirstF -eq 0) {
      [GCap]::Stop()
      Write-Host "      nothing arrived in 30 s" -ForegroundColor Red
      "### gesture: $($g.Name) [no keys in 30 s]" | Out-File $Out -Append -Encoding utf8
      $results += [pscustomobject]@{ Gesture = $g.Name; Presses = 0; SpanMs = "-"; Keys = ""; Verdict = "NONE" }
      continue
    }
    $stopAt = [GCap]::FirstF + ($Seconds * 1000)
    while ([Environment]::TickCount -lt $stopAt) { [GCap]::Pump(); Start-Sleep -Milliseconds 5 }
    [GCap]::Stop()
    $lines = @([GCap]::Lines)
    $header = "### gesture: $($g.Name)" + $(if ($g.Expect) { "  expect=$($g.Expect)" } else { "" })
    $header | Out-File $Out -Append -Encoding utf8
    $lines | Out-File $Out -Append -Encoding utf8
    # summary
    $fdown = $lines | Where-Object { $_ -match '\sDOWN F(1[3-9]|2[0-4])\s' }
    $keys = @($fdown | ForEach-Object { if ($_ -match 'DOWN (F\d+).*mods-at-this-instant=(\S+)') { if ($Matches[2] -eq '-') { $Matches[1] } else { "$($Matches[2])+$($Matches[1])" } } } | Sort-Object -Unique)
    $verdict = ""
    if ($g.Expect) { $verdict = if ($keys -contains $g.Expect -and $keys.Count -eq 1) { "MATCH" } else { "MISMATCH" } }
    Write-Host ("      {0} F-key press(es): {1}  {2}" -f $fdown.Count, ($keys -join ", "), $verdict) -ForegroundColor $(if ($verdict -eq "MISMATCH") { "Red" } else { "White" })
    Write-Host ""
    # span between the first and last F-key press, from the line timestamps
    $span = "-"
    if ($fdown.Count -gt 1) {
      $t0 = [datetime]::ParseExact($fdown[0].Substring(0, 12), "HH:mm:ss.fff", $null)
      $t1 = [datetime]::ParseExact($fdown[-1].Substring(0, 12), "HH:mm:ss.fff", $null)
      $span = [int]($t1 - $t0).TotalMilliseconds
    } elseif ($fdown.Count -eq 1) { $span = 0 }
    $shape = if ($fdown.Count -eq 0) { "NONE" } elseif ($fdown.Count -eq 1) { "ONCE" } else { "STREAM x$($fdown.Count)" }
    $results += [pscustomobject]@{ Gesture = $g.Name; Presses = $fdown.Count; SpanMs = $span; Keys = ($keys -join ", "); Verdict = (@($shape, $verdict) | Where-Object { $_ }) -join " " }
  }
} finally {
  [void][GCap]::UnhookWindowsHookEx([GCap]::Hook)
  if ($results.Count -gt 0) {
    Write-Host ""
    Write-Host "Summary: presses per gesture (ONCE = one key per gesture, STREAM = a run of keys from one gesture)" -ForegroundColor Cyan
    $results | Format-Table -AutoSize -Property @{ n = "Presses"; e = { $_.Presses }; a = "right" }, @{ n = "Span ms"; e = { $_.SpanMs }; a = "right" }, Keys, Verdict, Gesture | Out-String -Width 200 | Write-Host
    $streams = @($results | Where-Object { $_.Verdict -like "STREAM*" })
    $once = @($results | Where-Object { $_.Verdict -like "ONCE*" })
    Write-Host ("  {0} gesture(s) sent one key, {1} sent a run." -f $once.Count, $streams.Count) -ForegroundColor White
    if ($streams.Count -gt 0) { Write-Host ("  Runs: " + (($streams | ForEach-Object { "$($_.Gesture) ($($_.Presses))" }) -join "; ")) -ForegroundColor Yellow }
  }
  Write-Host "Done. Analyze with: python $analyzer $Out" -ForegroundColor Cyan
}
