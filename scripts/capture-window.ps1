param([string]$OutputPath = 'test-results\native-ui.png', [int]$StudioProcessId = 0)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class StudioWindow {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out Rect r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int command);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
}
'@
$studioProcess = if ($StudioProcessId) { Get-Process -Id $StudioProcessId } else {
    Get-Process -Name GridfinityStudioDesign2,gridfinity-studio-design2 -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Sort-Object StartTime -Descending | Select-Object -First 1
}
if (-not $studioProcess) { throw 'Kein Gridfinity-Studio-Fenster gefunden.' }
$windowHandle = $studioProcess.MainWindowHandle
$windowRect = New-Object StudioWindow+Rect
[StudioWindow]::GetWindowRect($windowHandle, [ref]$windowRect) | Out-Null
[StudioWindow]::ShowWindow($windowHandle, 9) | Out-Null
[StudioWindow]::SetForegroundWindow($windowHandle) | Out-Null
Start-Sleep -Milliseconds 250
$bitmap = New-Object System.Drawing.Bitmap(($windowRect.Right-$windowRect.Left), ($windowRect.Bottom-$windowRect.Top))
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($windowRect.Left, $windowRect.Top, 0, 0, $bitmap.Size)
$parent = Split-Path -Parent $OutputPath
if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
$bitmap.Save([IO.Path]::GetFullPath($OutputPath), [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bitmap.Dispose()
Write-Output ([IO.Path]::GetFullPath($OutputPath))
