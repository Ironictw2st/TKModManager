# Capture the TK Mod Manager window to a PNG (dev aid). Usage: powershell -File scripts\screenshot.ps1 out.png
param([string]$Out = "shot.png", [string]$Title = "TK Mod Manager")
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win32 {
  [DllImport("user32.dll")] public static extern IntPtr FindWindow(string cls, string title);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
}
"@
$proc = Get-Process | Where-Object { $_.MainWindowTitle -like "*$Title*" } | Select-Object -First 1
if (-not $proc) { Write-Error "window '$Title' not found"; exit 1 }
$h = $proc.MainWindowHandle
$r = New-Object Win32+RECT
[Win32]::GetWindowRect($h, [ref]$r) | Out-Null
$w = $r.R - $r.L; $hgt = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $hgt
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc()
[Win32]::PrintWindow($h, $hdc, 2) | Out-Null   # PW_RENDERFULLCONTENT
$g.ReleaseHdc($hdc)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
Write-Output "saved $Out ($w x $hgt)"
