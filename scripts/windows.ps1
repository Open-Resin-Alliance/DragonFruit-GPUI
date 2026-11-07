Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class W {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, UIntPtr e);
    public struct RECT { public int L, T, R, B; }
}
"@
Get-Process | Where-Object { $_.MainWindowHandle -ne 0 } | ForEach-Object {
    $r = New-Object W+RECT
    [W]::GetWindowRect($_.MainWindowHandle, [ref]$r) | Out-Null
    "{0} | {1} | {2},{3},{4},{5}" -f $_.ProcessName, $_.MainWindowTitle, $r.L, $r.T, $r.R, $r.B
}
