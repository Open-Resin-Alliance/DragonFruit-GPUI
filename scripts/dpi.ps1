param([string]$proc)
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class Dpi {
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
}
"@
$p = Get-Process $proc | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if ($p) {
    $h = $p.MainWindowHandle
    $r = New-Object Dpi+RECT
    [Dpi]::GetWindowRect($h, [ref]$r) | Out-Null
    Write-Output "$proc dpi=$([Dpi]::GetDpiForWindow($h)) rect=$($r.L),$($r.T),$($r.R),$($r.B)"
}
