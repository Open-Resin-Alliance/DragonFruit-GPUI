param([int]$x, [int]$y)
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class R {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
}
"@
[R]::SetCursorPos($x, $y) | Out-Null
Start-Sleep -Milliseconds 150
[R]::mouse_event(0x0008, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 60
[R]::mouse_event(0x0010, 0, 0, 0, [UIntPtr]::Zero)
