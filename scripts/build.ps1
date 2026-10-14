param([string]$TargetDir = '')
$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath (Split-Path -Parent $PSScriptRoot)
. (Join-Path $PSScriptRoot 'toolchain.ps1')
$cmakeDir = Split-Path -Parent (Find-GridfinityCmake)
$env:PATH = $cmakeDir + ';' + $env:PATH
$target = 'x86_64-pc-windows-msvc'
$buildArguments = @('build', '--locked', '--release', '--target', $target)
if ($TargetDir) { $buildArguments += @('--target-dir', $TargetDir) }
cargo @buildArguments
if ($LASTEXITCODE -ne 0) { throw 'Rust-Build fehlgeschlagen.' }
$buildDir = if ($TargetDir) { $TargetDir } elseif ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }
$buildExe = [System.IO.Path]::GetFullPath((Join-Path $buildDir "$target\release\gridfinitystudio.exe"))
Assert-GridfinityX64Pe -Path $buildExe
Copy-Item -LiteralPath $buildExe -Destination GridfinityStudio.exe
