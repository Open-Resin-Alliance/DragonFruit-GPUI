param([int]$x1, [int]$y1, [int]$x2, [int]$y2)
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class D {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
}
"@
[D]::SetCursorPos($x1, $y1) | Out-Null
Start-Sleep -Milliseconds 150
[D]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 100
$steps = 10
for ($i = 1; $i -le $steps; $i++) {
    $x = $x1 + [int](($x2 - $x1) * $i / $steps)
    $y = $y1 + [int](($y2 - $y1) * $i / $steps)
    [D]::SetCursorPos($x, $y) | Out-Null
    Start-Sleep -Milliseconds 30
}
[D]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
