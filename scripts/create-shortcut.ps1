$ErrorActionPreference = 'Stop'
$projectDir = Split-Path -Parent $PSScriptRoot
$assetsDir = Join-Path $projectDir 'assets'
New-Item -ItemType Directory -Force -Path $assetsDir | Out-Null
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class GridfinityIconNative {
    [DllImport("user32.dll")] public static extern bool DestroyIcon(IntPtr handle);
}
'@
$bitmap = New-Object System.Drawing.Bitmap 64,64
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.Clear([System.Drawing.Color]::FromArgb(247,245,239))
$brush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(30,108,91))
$pen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(247,245,239)),3
foreach ($x in @(9,34)) {
    foreach ($y in @(9,34)) {
        $graphics.FillRectangle($brush,$x,$y,21,21)
        $graphics.DrawRectangle($pen,$x+5,$y+5,11,11)
    }
}
$iconPath = Join-Path $assetsDir 'GridfinityStudioDesign2.ico'
$handle = $bitmap.GetHicon()
$icon = [System.Drawing.Icon]::FromHandle($handle)
$stream = [System.IO.File]::Create($iconPath)
try { $icon.Save($stream) } finally {
    $stream.Dispose()
    $icon.Dispose()
    [GridfinityIconNative]::DestroyIcon($handle) | Out-Null
    $pen.Dispose(); $brush.Dispose(); $graphics.Dispose(); $bitmap.Dispose()
}
$desktopDir = [Environment]::GetFolderPath('Desktop')
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut((Join-Path $desktopDir 'Gridfinity Studio - Design 2.lnk'))
$shortcut.TargetPath = Join-Path $projectDir 'GridfinityStudioDesign2.exe'
$shortcut.WorkingDirectory = $projectDir
$shortcut.IconLocation = $iconPath + ',0'
$shortcut.Description = 'Gridfinity Studio - Design 2: helle Werkstatt mit eigener Projektablage'
$shortcut.Save()
Write-Output (Join-Path $desktopDir 'Gridfinity Studio - Design 2.lnk')
