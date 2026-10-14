$ErrorActionPreference = 'Stop'
$projectDir = Split-Path -Parent $PSScriptRoot
$iconPath = Join-Path $projectDir 'assets\GridfinityStudio.ico'
$exePath = Join-Path $projectDir 'GridfinityStudio.exe'
if (-not (Test-Path -LiteralPath $exePath -PathType Leaf)) { throw 'GridfinityStudio.exe fehlt.' }
if (-not (Test-Path -LiteralPath $iconPath -PathType Leaf)) { throw 'Das App-Icon fehlt.' }
$desktopDir = [Environment]::GetFolderPath('Desktop')
$shell = New-Object -ComObject WScript.Shell
$shortcutPath = Join-Path $desktopDir 'GridfinityStudio.lnk'
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = $exePath
$shortcut.WorkingDirectory = $projectDir
$shortcut.IconLocation = $iconPath + ',0'
$shortcut.Description = 'GridfinityStudio - Gridfinity-Modelle entwerfen und als STL exportieren'
$shortcut.Save()
Write-Output $shortcutPath