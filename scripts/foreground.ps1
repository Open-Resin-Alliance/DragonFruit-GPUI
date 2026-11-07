Add-Type -TypeDefinition @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class G {
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT p);
    public struct POINT { public int X, Y; }
}
"@
$h = [G]::GetForegroundWindow()
$sb = New-Object System.Text.StringBuilder 256
[G]::GetWindowText($h, $sb, 256) | Out-Null
"foreground: {0} | {1}" -f $sb.ToString(), $h
$p = New-Object G+POINT
$p.X = 2500; $p.Y = 500
$w = [G]::WindowFromPoint($p)
$sb2 = New-Object System.Text.StringBuilder 256
[G]::GetWindowText($w, $sb2, 256) | Out-Null
"at 2500,500: {0} | {1}" -f $sb2.ToString(), $w
