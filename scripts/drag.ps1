# Dev aid: drag with the left mouse button between window-relative points in the TK Mod Manager window.
# Usage: powershell -File scripts\drag.ps1 -X1 300 -Y1 177 -X2 300 -Y2 220 -Process tkmm
param([int]$X1, [int]$Y1, [int]$X2, [int]$Y2, [string]$Title = "TK Mod Manager", [string]$Process = "")
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class DragW {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
}
"@
$proc = Get-Process | Where-Object { $_.MainWindowTitle -like "*$Title*" -and (-not $Process -or $_.ProcessName -eq $Process) } | Select-Object -First 1
if (-not $proc) { Write-Error "window '$Title' not found"; exit 1 }
$h = $proc.MainWindowHandle
[DragW]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 150
$r = New-Object DragW+RECT
[DragW]::GetWindowRect($h, [ref]$r) | Out-Null
[DragW]::SetCursorPos($r.L + $X1, $r.T + $Y1) | Out-Null
[DragW]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
$steps = 12
for ($i = 1; $i -le $steps; $i++) {
  Start-Sleep -Milliseconds 40
  [DragW]::SetCursorPos($r.L + $X1 + [int](($X2 - $X1) * $i / $steps), $r.T + $Y1 + [int](($Y2 - $Y1) * $i / $steps)) | Out-Null
}
Start-Sleep -Milliseconds 200
[DragW]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 500
Write-Output "dragged ($X1,$Y1) -> ($X2,$Y2)"
