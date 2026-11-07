param([string]$proc)
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class C {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
}
"@
$p = Get-Process $proc | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if ($p) {
    $h = $p.MainWindowHandle
    $r = New-Object C+RECT
    [C]::GetClientRect($h, [ref]$r) | Out-Null
    $pt = New-Object C+POINT
    [C]::ClientToScreen($h, [ref]$pt) | Out-Null
    Write-Output "$($pt.X),$($pt.Y),$($pt.X + $r.R),$($pt.Y + $r.B)"
}
