<#
.SYNOPSIS
  Print every keyboard event Windows delivers, the way Create Companion's hook sees it.

.DESCRIPTION
  Installs a low-level keyboard hook (WH_KEYBOARD_LL) and prints one line per key
  down/up: virtual key, scan code, whether the event was injected (SendInput), and
  the modifier state as GetAsyncKeyState reports it at that instant -- which is what
  the engine uses to decide whether F13 is "F13" or "Shift+F13".

  The hook passes everything through, so the engine keeps working while this runs
  (this hook was installed later, so it sees each key first). Run it in your own
  PowerShell window; Ctrl+C stops it.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File D:\CreateCompanion\tools\keymon.ps1
  powershell -ExecutionPolicy Bypass -File D:\CreateCompanion\tools\keymon.ps1 -Seconds 30 -OnlyFKeys
#>
param(
  [int]$Seconds = 0,      # 0 = until Ctrl+C
  [switch]$OnlyFKeys      # show only F13-F24 and the modifier keys
)

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class KeyMon {
  public delegate IntPtr HookProc(int code, IntPtr wParam, IntPtr lParam);
  [StructLayout(LayoutKind.Sequential)]
  public struct KBDLLHOOKSTRUCT { public uint vkCode; public uint scanCode; public uint flags; public uint time; public UIntPtr dwExtraInfo; }
  [StructLayout(LayoutKind.Sequential)]
  public struct MSG { public IntPtr hwnd; public uint message; public UIntPtr wParam; public IntPtr lParam; public uint time; public int x; public int y; }
  [DllImport("user32.dll", SetLastError = true)] public static extern IntPtr SetWindowsHookEx(int idHook, HookProc lpfn, IntPtr hMod, uint dwThreadId);
  [DllImport("user32.dll")] public static extern bool UnhookWindowsHookEx(IntPtr hhk);
  [DllImport("user32.dll")] public static extern IntPtr CallNextHookEx(IntPtr hhk, int nCode, IntPtr wParam, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool PeekMessage(out MSG msg, IntPtr hWnd, uint min, uint max, uint remove);
  [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int vk);
  [DllImport("kernel32.dll")] public static extern IntPtr GetModuleHandle(string name);

  public static HookProc Proc;   // keep the delegate alive
  public static IntPtr Hook;
  public static bool OnlyF;

  static bool Down(int vk) { return (GetAsyncKeyState(vk) & 0x8000) != 0; }
  static string Mods() {
    string s = "";
    if (Down(0x11)) s += "Ctrl+";
    if (Down(0x10)) s += "Shift+";
    if (Down(0x12)) s += "Alt+";
    if (Down(0x5B) || Down(0x5C)) s += "Win+";
    return s.Length == 0 ? "-" : s.TrimEnd('+');
  }
  static string Name(uint vk) {
    if (vk >= 0x70 && vk <= 0x87) return "F" + (vk - 0x70 + 1);
    switch (vk) {
      case 0xA0: return "LShift"; case 0xA1: return "RShift";
      case 0xA2: return "LCtrl";  case 0xA3: return "RCtrl";
      case 0xA4: return "LAlt";   case 0xA5: return "RAlt";
      case 0x5B: return "LWin";   case 0x5C: return "RWin";
      case 0x10: return "Shift";  case 0x11: return "Ctrl"; case 0x12: return "Alt";
    }
    try { return ((System.Windows.Forms.Keys)vk).ToString(); } catch { return "vk" + vk; }
  }
  public static IntPtr Callback(int code, IntPtr wParam, IntPtr lParam) {
    if (code == 0) {
      var k = (KBDLLHOOKSTRUCT)Marshal.PtrToStructure(lParam, typeof(KBDLLHOOKSTRUCT));
      bool up = (k.flags & 0x80) != 0;
      bool injected = (k.flags & 0x10) != 0;
      bool isF = k.vkCode >= 0x7C && k.vkCode <= 0x87;
      bool isMod = (k.vkCode >= 0xA0 && k.vkCode <= 0xA5) || k.vkCode == 0x5B || k.vkCode == 0x5C;
      if (!OnlyF || isF || isMod) {
        Console.WriteLine("{0:HH:mm:ss.fff}  {1,-4} {2,-8} vk=0x{3:X2} sc=0x{4:X3} {5} mods-at-this-instant={6}{7}",
          DateTime.Now, up ? "UP" : "DOWN", Name(k.vkCode), k.vkCode, k.scanCode,
          injected ? "INJECTED" : "physical", Mods(),
          isF ? (k.flags & 0x01) != 0 ? "  (extended)" : "" : "");
      }
    }
    return CallNextHookEx(Hook, code, wParam, lParam);
  }
  public static void Install(bool onlyF) {
    OnlyF = onlyF;
    Proc = new HookProc(Callback);
    Hook = SetWindowsHookEx(13, Proc, GetModuleHandle(null), 0);
    if (Hook == IntPtr.Zero) throw new Exception("SetWindowsHookEx failed: " + Marshal.GetLastWin32Error());
  }
  public static void Pump() {
    MSG m;
    while (PeekMessage(out m, IntPtr.Zero, 0, 0, 1)) { }
  }
}
'@ -ReferencedAssemblies System.Windows.Forms

[KeyMon]::Install([bool]$OnlyFKeys)
Write-Host "Listening for keys (hook installed). Press the module gestures now." -ForegroundColor Cyan
Write-Host "Columns: time, DOWN/UP, key, virtual key, scan code, physical|INJECTED, modifier state the engine would read." -ForegroundColor DarkGray
Write-Host ("Stop: " + $(if ($Seconds -gt 0) { "automatically after $Seconds s" } else { "Ctrl+C" })) -ForegroundColor DarkGray
$end = if ($Seconds -gt 0) { (Get-Date).AddSeconds($Seconds) } else { [DateTime]::MaxValue }
try {
  while ((Get-Date) -lt $end) {
    [KeyMon]::Pump()
    Start-Sleep -Milliseconds 5
  }
} finally {
  [void][KeyMon]::UnhookWindowsHookEx([KeyMon]::Hook)
  Write-Host "Hook removed." -ForegroundColor DarkGray
}
