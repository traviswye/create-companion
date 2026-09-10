<#
.SYNOPSIS
  Phase 1 capture: log EVERY keyboard and mouse event the PC receives while you
  perform each gesture on a module running its stock profile. No F-key flash needed.

.DESCRIPTION
  gesture_capture.ps1 answers "which F-key, and how many times" for a module flashed
  for Create Companion. This script comes before that: it hooks the keyboard AND the
  mouse (WH_KEYBOARD_LL + WH_MOUSE_LL) and records everything the module's stock
  profile sends - cursor moves with their deltas, wheel and horizontal-wheel notches,
  button clicks, and every key with the chord it forms (Ctrl+Tab, Win+Tab, ...).

  Guided mode (default): for each gesture in the plan it asks you to perform it, then
  records from the first event it sees until -Seconds (default 4) after it, echoes the
  stream live, and prints how many events of each kind arrived. A plan line may end in
  "expect=<tokens>" for a MATCH / MISMATCH verdict; tokens are move, wheel, hwheel,
  click:left|right|middle, a chord such as Ctrl+Shift+Tab, or none, separated by commas.
  A gesture expected to send nothing has no first event to wait for, so it is recorded
  for -NoneSeconds (default 8) from the moment you press Enter.
  The default plan is tools/plans/stock-touch.txt (the stock Windows Touch map).

  -Stream: no prompts; print every event live for -Seconds (0 = until you press Q),
  then the same summary. Use it for a first look, or to watch a gesture repeatedly.

  Output: a log with one "### gesture: <name>" header per gesture, ready for
  tools/input_analyze.py, and a summary table. The hooks pass everything through, so
  the gestures still DO what the stock profile says: taps are real clicks, 3- and
  4-finger swipes switch windows and desktops. Park the pointer over this console,
  keep your hands off the real mouse and keyboard during a capture, and click back
  into the console if a gesture takes the focus away. Run from the repository root.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File tools\input_capture.ps1
  powershell -ExecutionPolicy Bypass -File tools\input_capture.ps1 -PlanFile tools\plans\stock-touch.txt -Seconds 4 -Out tools\plans\stock-touch.log
  powershell -ExecutionPolicy Bypass -File tools\input_capture.ps1 -Only "3 fingers" -Append
  powershell -ExecutionPolicy Bypass -File tools\input_capture.ps1 -Gestures "2-finger scroll up","1-finger tap"
  powershell -ExecutionPolicy Bypass -File tools\input_capture.ps1 -Stream -Seconds 20
  python tools\input_analyze.py tools\plans\stock-touch.log
#>
param(
  [string]$PlanFile,        # one gesture per line, optional "expect=..." suffix; default tools\plans\stock-touch.txt
  [string[]]$Gestures,      # prompts given on the command line instead of a plan file
  [int]$Seconds = 4,        # record this long after the first event of each gesture
  [int]$NoneSeconds = 8,    # gestures expected to send nothing: record this long from Enter instead
  [string]$Out,             # log file; default: input.log in the repository root
  [string[]]$Only,          # name fragments to test, e.g. "3 fingers","tap (1"
  [switch]$Append,          # add to the log instead of starting a new one
  [switch]$Stream,          # no prompts: log everything for -Seconds (0 = until Q)
  [switch]$IncludeInjected, # also log SendInput events (self-test without a device)
  [switch]$Quiet,           # no live echo; summaries only
  [switch]$DryRun           # print the plan and exit
)
$repoRoot = Split-Path $PSScriptRoot -Parent
if (-not $Out) { $Out = Join-Path $repoRoot "input.log" }
if (-not $PlanFile -and -not $Gestures -and -not $Stream) { $PlanFile = Join-Path $PSScriptRoot "plans\stock-touch.txt" }
$analyzer = Join-Path $PSScriptRoot "input_analyze.py"

Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class ICap {
  public delegate IntPtr HookProc(int code, IntPtr wParam, IntPtr lParam);
  [StructLayout(LayoutKind.Sequential)]
  public struct KBDLLHOOKSTRUCT { public uint vkCode; public uint scanCode; public uint flags; public uint time; public UIntPtr dwExtraInfo; }
  [StructLayout(LayoutKind.Sequential)]
  public struct POINT { public int x; public int y; }
  [StructLayout(LayoutKind.Sequential)]
  public struct MSLLHOOKSTRUCT { public POINT pt; public uint mouseData; public uint flags; public uint time; public UIntPtr dwExtraInfo; }
  [StructLayout(LayoutKind.Sequential)]
  public struct MSG { public IntPtr hwnd; public uint message; public UIntPtr wParam; public IntPtr lParam; public uint time; public int x; public int y; }
  [DllImport("user32.dll", SetLastError = true)] public static extern IntPtr SetWindowsHookEx(int idHook, HookProc lpfn, IntPtr hMod, uint dwThreadId);
  [DllImport("user32.dll")] public static extern bool UnhookWindowsHookEx(IntPtr hhk);
  [DllImport("user32.dll")] public static extern IntPtr CallNextHookEx(IntPtr hhk, int nCode, IntPtr wParam, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool PeekMessage(out MSG msg, IntPtr hWnd, uint min, uint max, uint remove);
  [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int vk);
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  [DllImport("kernel32.dll")] public static extern IntPtr GetModuleHandle(string name);

  public static HookProc KbProc, MsProc;   // keep the delegates alive
  public static IntPtr KbHook, MsHook;
  public static List<string> Lines = new List<string>();
  public static bool Recording = false;
  public static bool IncludeInjected = false;
  public static long FirstTick = 0;        // tick of the first recorded event, 0 = none yet
  static int lastX, lastY;                 // cursor position before the current event, for deltas
  static readonly Dictionary<int, bool> modDown = new Dictionary<int, bool>();

  public static bool KeyIsDown(int vk) { return (GetAsyncKeyState(vk) & 0x8000) != 0; }
  public static void Reset() { Lines.Clear(); FirstTick = 0; Recording = true; }
  public static void Stop() { Recording = false; }

  static bool IsMod(uint vk) { return (vk >= 0xA0 && vk <= 0xA5) || vk == 0x5B || vk == 0x5C || vk == 0x10 || vk == 0x11 || vk == 0x12; }
  static string Name(uint vk) {
    if (vk >= 0x70 && vk <= 0x87) return "F" + (vk - 0x70 + 1);
    if ((vk >= 0x30 && vk <= 0x39) || (vk >= 0x41 && vk <= 0x5A)) return ((char)vk).ToString();
    if (vk >= 0x60 && vk <= 0x69) return "Num" + (vk - 0x60);
    switch (vk) {
      case 0xA0: return "LShift"; case 0xA1: return "RShift"; case 0xA2: return "LCtrl"; case 0xA3: return "RCtrl";
      case 0xA4: return "LAlt"; case 0xA5: return "RAlt"; case 0x5B: return "LWin"; case 0x5C: return "RWin";
      case 0x10: return "Shift"; case 0x11: return "Ctrl"; case 0x12: return "Alt";
      case 0x08: return "Backspace"; case 0x09: return "Tab"; case 0x0D: return "Enter"; case 0x1B: return "Esc"; case 0x20: return "Space";
      case 0x21: return "PgUp"; case 0x22: return "PgDn"; case 0x23: return "End"; case 0x24: return "Home";
      case 0x25: return "Left"; case 0x26: return "Up"; case 0x27: return "Right"; case 0x28: return "Down";
      case 0x2C: return "PrintScreen"; case 0x2D: return "Ins"; case 0x2E: return "Del"; case 0x5D: return "Apps";
      case 0x14: return "CapsLock"; case 0x90: return "NumLock"; case 0x91: return "ScrollLock";
      case 0xAD: return "VolumeMute"; case 0xAE: return "VolumeDown"; case 0xAF: return "VolumeUp";
      case 0xB0: return "MediaNext"; case 0xB1: return "MediaPrev"; case 0xB2: return "MediaStop"; case 0xB3: return "MediaPlayPause";
      case 0xA6: return "BrowserBack"; case 0xA7: return "BrowserForward";
    }
    return string.Format("VK_0x{0:X2}", vk);
  }
  // Modifiers currently held (physical, or injected when IncludeInjected), in a fixed order.
  static string Held() {
    bool ctrl = false, shift = false, alt = false, win = false;
    foreach (var kv in modDown) {
      if (!kv.Value) continue;
      int v = kv.Key;
      if (v == 0xA2 || v == 0xA3 || v == 0x11) ctrl = true;
      else if (v == 0xA0 || v == 0xA1 || v == 0x10) shift = true;
      else if (v == 0xA4 || v == 0xA5 || v == 0x12) alt = true;
      else if (v == 0x5B || v == 0x5C) win = true;
    }
    string s = "";
    if (ctrl) s += "Ctrl+"; if (shift) s += "Shift+"; if (alt) s += "Alt+"; if (win) s += "Win+";
    return s;
  }
  static string Stamp() {
    if (FirstTick == 0) FirstTick = Environment.TickCount;
    long rel = Environment.TickCount - FirstTick;
    return string.Format("{0:HH:mm:ss.fff}  +{1,5}ms  ", DateTime.Now, rel);
  }
  public static IntPtr KbCallback(int code, IntPtr wParam, IntPtr lParam) {
    if (code == 0) {
      var k = (KBDLLHOOKSTRUCT)Marshal.PtrToStructure(lParam, typeof(KBDLLHOOKSTRUCT));
      bool up = (k.flags & 0x80) != 0;
      bool injected = (k.flags & 0x10) != 0;
      if (!injected || IncludeInjected) {
        if (IsMod(k.vkCode)) modDown[(int)k.vkCode] = !up;
        if (Recording) {
          string chord = (!up && !IsMod(k.vkCode)) ? "  chord=" + Held() + Name(k.vkCode) : "";
          Lines.Add(Stamp() + string.Format("KEY   {0,-4} {1,-14} vk=0x{2:X2} sc=0x{3:X3}{4}{5}",
            up ? "UP" : "DOWN", Name(k.vkCode), k.vkCode, k.scanCode, chord, injected ? "  INJECTED" : ""));
        }
      }
    }
    return CallNextHookEx(KbHook, code, wParam, lParam);
  }
  public static IntPtr MsCallback(int code, IntPtr wParam, IntPtr lParam) {
    if (code == 0) {
      var m = (MSLLHOOKSTRUCT)Marshal.PtrToStructure(lParam, typeof(MSLLHOOKSTRUCT));
      uint msg = (uint)wParam.ToInt64();
      bool injected = (m.flags & 0x01) != 0;
      int dx = m.pt.x - lastX, dy = m.pt.y - lastY;
      if (msg == 0x200) { lastX = m.pt.x; lastY = m.pt.y; }
      if ((!injected || IncludeInjected) && Recording) {
        short hi = (short)((m.mouseData >> 16) & 0xFFFF);   // wheel delta, or X button number
        string what;
        switch (msg) {
          case 0x200: what = string.Format("MOUSE MOVE   x={0},y={1}  dx={2},dy={3}", m.pt.x, m.pt.y, dx, dy); break;
          case 0x201: what = "MOUSE DOWN Left"; break;    case 0x202: what = "MOUSE UP   Left"; break;
          case 0x204: what = "MOUSE DOWN Right"; break;   case 0x205: what = "MOUSE UP   Right"; break;
          case 0x207: what = "MOUSE DOWN Middle"; break;  case 0x208: what = "MOUSE UP   Middle"; break;
          case 0x20B: what = "MOUSE DOWN X" + hi; break;  case 0x20C: what = "MOUSE UP   X" + hi; break;
          case 0x20A: what = string.Format("MOUSE WHEEL  delta={0:+#;-#;0}", hi); break;
          case 0x20E: what = string.Format("MOUSE HWHEEL delta={0:+#;-#;0}", hi); break;
          default: what = string.Format("MOUSE msg=0x{0:X}", msg); break;
        }
        Lines.Add(Stamp() + what + (injected ? "  INJECTED" : ""));
      }
    }
    return CallNextHookEx(MsHook, code, wParam, lParam);
  }
  public static void Install(bool includeInjected) {
    IncludeInjected = includeInjected;
    POINT p; if (GetCursorPos(out p)) { lastX = p.x; lastY = p.y; }
    KbProc = new HookProc(KbCallback);
    MsProc = new HookProc(MsCallback);
    IntPtr mod = GetModuleHandle(null);
    KbHook = SetWindowsHookEx(13, KbProc, mod, 0);
    if (KbHook == IntPtr.Zero) throw new Exception("keyboard hook failed: " + Marshal.GetLastWin32Error());
    MsHook = SetWindowsHookEx(14, MsProc, mod, 0);
    if (MsHook == IntPtr.Zero) throw new Exception("mouse hook failed: " + Marshal.GetLastWin32Error());
  }
  public static void Uninstall() {
    if (KbHook != IntPtr.Zero) UnhookWindowsHookEx(KbHook);
    if (MsHook != IntPtr.Zero) UnhookWindowsHookEx(MsHook);
  }
  public static void Pump() { MSG m; while (PeekMessage(out m, IntPtr.Zero, 0, 0, 1)) { } }
}
'@

# ---- expectation tokens: "Alt+Shift+Esc" and "shift+alt+escape" both become "Shift+Alt+Esc" ----
$keyAliases = @{
  'escape' = 'Esc'; 'esc' = 'Esc'; 'pageup' = 'PgUp'; 'pg_up' = 'PgUp'; 'pgup' = 'PgUp'; 'pagedown' = 'PgDn'; 'pg_dn' = 'PgDn'; 'pgdn' = 'PgDn'
  'return' = 'Enter'; 'enter' = 'Enter'; 'delete' = 'Del'; 'del' = 'Del'; 'insert' = 'Ins'; 'ins' = 'Ins'; 'tab' = 'Tab'; 'space' = 'Space'
  'left' = 'Left'; 'right' = 'Right'; 'up' = 'Up'; 'down' = 'Down'; 'home' = 'Home'; 'end' = 'End'; 'backspace' = 'Backspace'
}
function Normalize([string]$tok) {
  $t = $tok.Trim()
  if (-not $t) { return $null }
  $l = $t.ToLower()
  if ($l -in @('move', 'wheel', 'hwheel', 'none') -or $l -like 'click:*') { return $l }
  $mods = @{}; $key = $null
  foreach ($p in ($t -split '\+' | ForEach-Object { $_.Trim() } | Where-Object { $_ })) {
    switch -regex ($p.ToLower()) {
      '^(ctrl|control|lctrl|rctrl)$'                       { $mods['Ctrl'] = $true }
      '^(shift|lshift|rshift)$'                            { $mods['Shift'] = $true }
      '^(alt|lalt|ralt|option)$'                           { $mods['Alt'] = $true }
      '^(win|lwin|rwin|windows|gui|lgui|rgui|cmd|meta)$'   { $mods['Win'] = $true }
      default                                              { $key = $p }
    }
  }
  if ($key) {
    $kl = $key.ToLower()
    if ($keyAliases.ContainsKey($kl)) { $key = $keyAliases[$kl] } elseif ($key.Length -eq 1) { $key = $key.ToUpper() }
  }
  $s = ''
  foreach ($m in 'Ctrl', 'Shift', 'Alt', 'Win') { if ($mods[$m]) { $s += "$m+" } }
  return $s + $key
}

# ---- what a capture contained, as counts and as tokens for the verdict ----
function Summarize($lines) {
  $chords = @{}; $clicks = @{}
  $wheelN = 0; $wheelD = 0; $hwheelN = 0; $hwheelD = 0; $moves = 0; $dx = 0; $dy = 0
  foreach ($l in $lines) {
    if ($l -match 'KEY\s+DOWN\s+\S+.*chord=(\S+)') { $c = Normalize $Matches[1]; $chords[$c] = 1 + [int]$chords[$c] }
    elseif ($l -match 'MOUSE MOVE\s+x=-?\d+,y=-?\d+\s+dx=(-?\d+),dy=(-?\d+)') { $moves++; $dx += [int]$Matches[1]; $dy += [int]$Matches[2] }
    elseif ($l -match 'MOUSE WHEEL\s+delta=([+-]?\d+)') { $wheelN++; $wheelD += [int]$Matches[1] }
    elseif ($l -match 'MOUSE HWHEEL\s+delta=([+-]?\d+)') { $hwheelN++; $hwheelD += [int]$Matches[1] }
    elseif ($l -match 'MOUSE DOWN\s+(\S+)') { $b = $Matches[1].ToLower(); $clicks[$b] = 1 + [int]$clicks[$b] }
  }
  $tokens = @()
  $parts = @()
  if ($chords.Count -gt 0) {
    $tokens += @($chords.Keys)
    $parts += "keys: " + (($chords.GetEnumerator() | Sort-Object Name | ForEach-Object { "$($_.Key) x$($_.Value)" }) -join ", ")
  }
  if ($clicks.Count -gt 0) {
    $tokens += @($clicks.Keys | ForEach-Object { "click:$_" })
    $parts += "clicks: " + (($clicks.GetEnumerator() | Sort-Object Name | ForEach-Object { "$($_.Key) x$($_.Value)" }) -join ", ")
  }
  if ($wheelN -gt 0) { $tokens += 'wheel'; $parts += ("wheel: {0} event(s), {1:0.##} notch(es) (delta {2:+#;-#;0})" -f $wheelN, ($wheelD / 120), $wheelD) }
  if ($hwheelN -gt 0) { $tokens += 'hwheel'; $parts += ("hwheel: {0} event(s), {1:0.##} notch(es) (delta {2:+#;-#;0})" -f $hwheelN, ($hwheelD / 120), $hwheelD) }
  if ($moves -gt 0) { $tokens += 'move'; $parts += ("moves: {0} (dx {1:+#;-#;0}, dy {2:+#;-#;0})" -f $moves, $dx, $dy) }
  [pscustomobject]@{ Tokens = $tokens; Text = $(if ($parts.Count -gt 0) { $parts -join " | " } else { "nothing" }); Events = $lines.Count }
}
function Verdict($expect, $observed) {
  if (-not $expect) { return "" }
  $exp = @(($expect -split ',') | ForEach-Object { Normalize $_ } | Where-Object { $_ })
  if ($exp -contains 'none') { return $(if ($observed.Count -eq 0) { "MATCH" } else { "MISMATCH (expected nothing)" }) }
  $missing = @($exp | Where-Object { $observed -notcontains $_ })
  $extra = @($observed | Where-Object { $exp -notcontains $_ })
  $v = if ($missing.Count -eq 0) { "MATCH" } else { "MISMATCH (missing " + ($missing -join ", ") + ")" }
  if ($extra.Count -gt 0) { $v += " +extra " + ($extra -join ", ") }
  return $v
}
function SpanMs($lines) {
  if ($lines.Count -lt 2) { return 0 }
  if ($lines[-1] -match '\+\s*(\d+)ms') { return [int]$Matches[1] } else { return 0 }
}

# ---- live echo of new lines since the last call ----
$script:shown = 0
function Flush {
  $n = [ICap]::Lines.Count
  if ($Quiet -or $n -le $script:shown) { return }
  $chunk = for ($i = $script:shown; $i -lt $n; $i++) { "      " + [ICap]::Lines[$i] }
  Write-Host ($chunk -join "`n") -ForegroundColor DarkGray
  $script:shown = $n
}
function WaitKey {
  while (-not [Console]::KeyAvailable) { [ICap]::Pump(); Start-Sleep -Milliseconds 10 }
  return [Console]::ReadKey($true)
}
function Record([int]$seconds, [int]$waitFirst, [bool]$startNow = $false) {
  # Record from the first event until $seconds after it, giving up after $waitFirst s of
  # silence; or, with $startNow, record for $seconds from right now (a gesture expected to
  # send nothing has no first event to wait for). Returns $null when nothing arrived.
  $script:shown = 0
  [ICap]::Reset()
  if ($startNow) {
    $stopAt = [Environment]::TickCount + ($seconds * 1000)
  } else {
    $deadline = (Get-Date).AddSeconds($waitFirst)
    while ([ICap]::FirstTick -eq 0 -and (Get-Date) -lt $deadline) { [ICap]::Pump(); Start-Sleep -Milliseconds 5 }
    if ([ICap]::FirstTick -eq 0) { [ICap]::Stop(); return $null }
    $stopAt = [ICap]::FirstTick + ($seconds * 1000)
  }
  while ([Environment]::TickCount -lt $stopAt) { [ICap]::Pump(); Flush; Start-Sleep -Milliseconds 5 }
  [ICap]::Stop()
  Flush
  $lines = @([ICap]::Lines)
  if ($lines.Count -eq 0) { return $null }
  return ,$lines
}

# ---- the plan ----
$plan = @()
if ($Stream) {
  # nothing to plan
} elseif ($Gestures) {
  foreach ($g in @($Gestures | Where-Object { $_.Trim() })) { $plan += [pscustomobject]@{ Name = $g.Trim(); Expect = $null } }
} else {
  if (-not (Test-Path $PlanFile)) { Write-Host "Plan file not found: $PlanFile" -ForegroundColor Red; exit 1 }
  foreach ($line in Get-Content $PlanFile) {
    $t = $line.Trim()
    if (-not $t -or $t.StartsWith("#")) { continue }
    $expect = $null
    if ($t -match '^(.*?)\s+expect=(\S+)$') { $t = $Matches[1]; $expect = $Matches[2] }
    $plan += [pscustomobject]@{ Name = $t; Expect = $expect }
  }
}
if ($Only -and -not $Stream) {
  $Only = @($Only | ForEach-Object { $_ -split "," } | Where-Object { $_ })
  $plan = @($plan | Where-Object { $g = $_; ($Only | Where-Object { $g.Name -like "*$_*" }).Count -gt 0 })
  if ($plan.Count -eq 0) { Write-Host "Nothing matches -Only $($Only -join ', ')" -ForegroundColor Red; exit 1 }
}
if ($DryRun) {
  if ($Stream) { Write-Host "Stream mode: $(if ($Seconds -gt 0) { "$Seconds s" } else { 'until Q' }), log $Out"; exit 0 }
  Write-Host "Plan ($($plan.Count) gestures):"
  $plan | ForEach-Object { Write-Host ("  {0,-70} {1}" -f $_.Name, $_.Expect) }
  exit 0
}

[ICap]::Install([bool]$IncludeInjected)
$stamp = "# input capture $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')  post-first-event window ${Seconds}s$(if ($IncludeInjected) { '  (injected events included)' })"
if ($Append) { $stamp | Out-File $Out -Append -Encoding utf8 } else { $stamp | Out-File $Out -Encoding utf8 }
$results = @()
try {
  if ($Stream) {
    Write-Host "Streaming every keyboard and mouse event for $(if ($Seconds -gt 0) { "$Seconds s" } else { 'as long as you like; press Q to stop' }). Log: $Out" -ForegroundColor Cyan
    $script:shown = 0
    [ICap]::Reset()
    $end = if ($Seconds -gt 0) { (Get-Date).AddSeconds($Seconds) } else { [DateTime]::MaxValue }
    while ((Get-Date) -lt $end) {
      [ICap]::Pump(); Flush; Start-Sleep -Milliseconds 5
      try { if ([Console]::KeyAvailable) { $k = [Console]::ReadKey($true); if ($k.Key -eq "Q") { break } } } catch { }
    }
    [ICap]::Stop(); Flush
    $lines = @([ICap]::Lines)
    "### stream $(Get-Date -Format 'HH:mm:ss')" | Out-File $Out -Append -Encoding utf8
    $lines | Out-File $Out -Append -Encoding utf8
    $s = Summarize $lines
    "# summary: $($s.Text)" | Out-File $Out -Append -Encoding utf8
    $results += [pscustomobject]@{ Gesture = "stream"; Events = $s.Events; SpanMs = (SpanMs $lines); Observed = $s.Text; Verdict = "" }
  } else {
    Write-Host "Guided capture: $($plan.Count) gestures, $Seconds s after the first event each. Log: $Out" -ForegroundColor Cyan
    Write-Host "Park the pointer over this window; hands off the real mouse and keyboard during a capture." -ForegroundColor DarkGray
    Write-Host "At each prompt: press Enter, then perform the gesture ONCE. S = skip, Q = quit." -ForegroundColor DarkGray
    Write-Host ""
    $n = 0
    foreach ($g in $plan) {
      $n++
      $exp = if ($g.Expect) { "  (stock: $($g.Expect))" } else { "" }
      Write-Host ("[{0}/{1}] Next: {2}{3}" -f $n, $plan.Count, $g.Name, $exp) -ForegroundColor Yellow
      Write-Host "      Enter = ready, S = skip, Q = quit" -ForegroundColor DarkGray
      $k = WaitKey
      if ($k.Key -eq "Q") { break }
      if ($k.Key -eq "S") {
        "### gesture: $($g.Name) [skipped]" | Out-File $Out -Append -Encoding utf8
        $results += [pscustomobject]@{ Gesture = $g.Name; Events = "-"; SpanMs = "-"; Observed = ""; Verdict = "skipped" }
        continue
      }
      # the Enter that started this capture must be released before we listen, or its UP is the first event
      while ([ICap]::KeyIsDown(0x0D)) { [ICap]::Pump(); Start-Sleep -Milliseconds 5 }
      $expectNothing = [bool]($g.Expect -and ((($g.Expect -split ',') | ForEach-Object { $_.Trim().ToLower() }) -contains 'none'))
      if ($expectNothing) {
        Write-Host "      Go: perform the gesture now; recording for $NoneSeconds s from this moment..." -ForegroundColor Green
        $lines = Record $NoneSeconds 0 $true
      } else {
        Write-Host "      Go: perform the gesture once..." -ForegroundColor Green
        $lines = Record $Seconds 30 $false
      }
      $header = "### gesture: $($g.Name)" + $(if ($g.Expect) { "  expect=$($g.Expect)" } else { "" })
      if ($null -eq $lines) {
        $wait = if ($expectNothing) { $NoneSeconds } else { 30 }
        $v = Verdict $g.Expect @()
        Write-Host "      nothing arrived in $wait s  $v" -ForegroundColor $(if ($v -like "MISMATCH*") { "Red" } else { "White" })
        "$header [no input in $wait s]" | Out-File $Out -Append -Encoding utf8
        $results += [pscustomobject]@{ Gesture = $g.Name; Events = 0; SpanMs = "-"; Observed = "nothing"; Verdict = $v }
        Write-Host ""
        continue
      }
      $header | Out-File $Out -Append -Encoding utf8
      $lines | Out-File $Out -Append -Encoding utf8
      $s = Summarize $lines
      "# summary: $($s.Text)" | Out-File $Out -Append -Encoding utf8
      $v = Verdict $g.Expect $s.Tokens
      Write-Host ("      {0} event(s) over {1} ms: {2}  {3}" -f $s.Events, (SpanMs $lines), $s.Text, $v) -ForegroundColor $(if ($v -like "MISMATCH*") { "Red" } else { "White" })
      Write-Host ""
      $results += [pscustomobject]@{ Gesture = $g.Name; Events = $s.Events; SpanMs = (SpanMs $lines); Observed = $s.Text; Verdict = $v }
    }
  }
} finally {
  [ICap]::Uninstall()
  if ($results.Count -gt 0) {
    Write-Host ""
    Write-Host "Summary: what the PC received per gesture" -ForegroundColor Cyan
    $results | Format-Table -AutoSize -Wrap -Property @{ n = "Events"; e = { $_.Events }; a = "right" }, @{ n = "Span ms"; e = { $_.SpanMs }; a = "right" }, Gesture, Observed, Verdict | Out-String -Width 220 | Write-Host
    $bad = @($results | Where-Object { $_.Verdict -like "MISMATCH*" })
    if ($bad.Count -gt 0) { Write-Host ("  {0} gesture(s) did not match the stock map: {1}" -f $bad.Count, (($bad | ForEach-Object { $_.Gesture }) -join "; ")) -ForegroundColor Yellow }
  }
  Write-Host "Done. Analyze with: python $analyzer $Out" -ForegroundColor Cyan
}
