param([string]$proc, [int]$x, [int]$y, [int]$w, [int]$h)
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class P {
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
}
"@
$p = Get-Process $proc | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if ($p) {
    $h = $p.MainWindowHandle
    [P]::ShowWindow($h, 9) | Out-Null
    [P]::SetWindowPos($h, [IntPtr](-1), $x, $y, $w, $h, 0x0040) | Out-Null
    [P]::SetWindowPos($h, [IntPtr](-2), $x, $y, $w, $h, 0x0040) | Out-Null
    [P]::BringWindowToTop($h) | Out-Null
    [P]::SetForegroundWindow($h) | Out-Null
}
