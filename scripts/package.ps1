param([string]$OutputPath = '', [string]$BuildExe = '', [string]$CrtDir = '')
$ErrorActionPreference = 'Stop'
$projectDir = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'toolchain.ps1')
if (-not $OutputPath) {
    $OutputPath = Join-Path ([Environment]::GetFolderPath('Desktop')) ('GridfinityStudio-v' + ((Get-Content (Join-Path $projectDir 'Cargo.toml') | Select-String '^version = "([^"]+)"').Matches.Groups[1].Value) + '-Windows-x64.zip')
}
if (Test-Path -LiteralPath $OutputPath) { throw "ZIP existiert bereits: $OutputPath" }
if (-not $BuildExe) {
    $targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }
    if (-not [System.IO.Path]::IsPathRooted($targetDir)) { $targetDir = Join-Path $projectDir $targetDir }
    $BuildExe = Join-Path $targetDir 'x86_64-pc-windows-msvc\release\gridfinitystudio.exe'
    if (-not (Test-Path -LiteralPath $BuildExe -PathType Leaf)) {
        # Builds made before the explicit target was introduced remain usable.
        $BuildExe = Join-Path $targetDir 'release\gridfinitystudio.exe'
    }
}
if (-not (Test-Path -LiteralPath $BuildExe -PathType Leaf)) { throw "GridfinityStudio-Build fehlt: $BuildExe" }
$BuildExe = (Resolve-Path -LiteralPath $BuildExe).ProviderPath
Assert-GridfinityX64Pe -Path $BuildExe
$resolvedCrtDir = Resolve-GridfinityCrtDirectory -CrtDir $CrtDir
$packageDir = Join-Path $projectDir ('dist\portable-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [Guid]::NewGuid().ToString('N').Substring(0, 8))
$appDir = Join-Path $packageDir 'GridfinityStudio'
New-Item -ItemType Directory -Force $appDir | Out-Null
Copy-Item -LiteralPath $BuildExe -Destination (Join-Path $appDir 'GridfinityStudio.exe')
New-Item -ItemType Directory (Join-Path $appDir 'examples') | Out-Null
Get-ChildItem -LiteralPath (Join-Path $projectDir 'examples') -Filter '*.gridfinity.json' -File | Copy-Item -Destination (Join-Path $appDir 'examples')
foreach ($item in @('assets','docs','licenses','README.md','PRINTERS.md','SECURITY-REVIEW.md','THIRD-PARTY-NOTICES.md','LICENSE')) {
    Copy-Item -LiteralPath (Join-Path $projectDir $item) -Destination $appDir -Recurse
}
New-Item -ItemType Directory (Join-Path $appDir 'scripts') | Out-Null
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'create-shortcut.ps1') -Destination (Join-Path $appDir 'scripts')
Get-ChildItem -LiteralPath $resolvedCrtDir -File -Filter '*.dll' | Copy-Item -Destination $appDir
@('@echo off','cd /d "%~dp0"','start "" "%~dp0GridfinityStudio.exe"') | Set-Content (Join-Path $appDir 'Start.cmd') -Encoding ascii
@('@echo off','cd /d "%~dp0"','powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\create-shortcut.ps1"','pause') | Set-Content (Join-Path $appDir 'Desktop-Icon-erstellen.cmd') -Encoding ascii
@('GridfinityStudio | portable Windows-Version (x64)', '',
  '1. Die gesamte ZIP entpacken.',
  '2. GridfinityStudio.exe oder Start.cmd starten.',
  '3. Unter Start links den Modelltyp wählen und rechts auf Erstellen klicken.',
  'Beispiele stehen unter Datei > Beispiele.',
  'Links: Start, Entwurf, Projekte, Drucker und Hilfe.',
  'Rechts im Editor stehen direkt die Eigenschaften des Modells.',
  'Unten links mit Hell / Dunkel zwischen Grauweiß und Graphit wechseln.', '',
  'Beschriftung: direkter Text, blankes Etikettenfeld oder Schildaufnahme unten an der Front.',
  'Einschubschilder separat drucken und von oben in die Führung schieben.',
  'Unter Export das gewünschte Druckteil auswählen.',
  'Projekte in der linken Leiste öffnet die Ablage für Projekte und Unterordner.',
  'Projekte, Ordner und einzelne Entwürfe lassen sich dort mit Bestätigung löschen.',
  'Links unter Drucker ein Profil oder eigene Bauraummaße einstellen.',
  'Druckerprüfung und Profilquellen: PRINTERS.md.',
  'Projekte und Einstellungen bleiben bei Programmupdates erhalten.',
  'Die Projektablage liegt unter %LOCALAPPDATA%\GridfinityStudioDesign2\Projekte.',
  'Hell/Dunkel wird in %LOCALAPPDATA%\GridfinityStudioDesign2\appearance.json gespeichert.',
  'Entwürfe werden als .gridfinity.json gespeichert.',
  'Die Programm-ZIP enthält keine persönlichen Entwürfe.', '',
  'Optional: Desktop-Icon-erstellen.cmd legt eine Verknüpfung an.',
  'Voraussetzungen: Windows 10/11, 64 Bit, DirectX 12.',
  'Microsoft Visual C++ Runtime DLLs sind proprietäre Microsoft-Komponenten.',
  'Eigene Werkzeug-STLs separat übertragen und gegebenenfalls neu auswählen.',
  'Druck- und Clip-Hinweise: README.md.') | Set-Content (Join-Path $appDir 'HIER-STARTEN.txt') -Encoding utf8
# ZIP timestamps cannot represent dates before 1980 (some crates use Unix epoch).
Get-ChildItem -LiteralPath $appDir -Recurse -File | Where-Object { $_.LastWriteTime.Year -lt 1980 } | ForEach-Object { $_.LastWriteTime = [datetime]'2000-01-01' }
Compress-Archive -LiteralPath $appDir -DestinationPath $OutputPath -CompressionLevel Optimal
Write-Output $OutputPath
