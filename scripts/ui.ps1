# Dev aid: click at window-relative coordinates in the TK Mod Manager window and optionally type.
# Usage: powershell -File scripts\ui.ps1 -X 100 -Y 96 [-Text "se_"] [-Keys "{ENTER}"]
param([int]$X, [int]$Y, [string]$Text = "", [string]$Keys = "", [string]$Title = "TK Mod Manager")
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class UiW {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
}
"@
$proc = Get-Process | Where-Object { $_.MainWindowTitle -like "*$Title*" } | Select-Object -First 1
if (-not $proc) { Write-Error "window '$Title' not found"; exit 1 }
$h = $proc.MainWindowHandle
[UiW]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 150
$r = New-Object UiW+RECT
[UiW]::GetWindowRect($h, [ref]$r) | Out-Null
[UiW]::SetCursorPos($r.L + $X, $r.T + $Y) | Out-Null
[UiW]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
[UiW]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 150
$ws = New-Object -ComObject WScript.Shell
if ($Text) { $ws.SendKeys($Text) }
if ($Keys) { $ws.SendKeys($Keys) }
Start-Sleep -Milliseconds 400
Write-Output "clicked ($X,$Y)"
