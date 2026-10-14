# Shared discovery for Windows builds and portable packages. No fixed VS version.
function Get-GridfinityVisualStudioRoots {
    $roots = @()
    if ($env:VSINSTALLDIR -and (Test-Path -LiteralPath $env:VSINSTALLDIR -PathType Container)) {
        $roots += $env:VSINSTALLDIR
    }
    $vswhere = Get-Command vswhere.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    $vswherePath = if ($vswhere) { $vswhere.Source } else { '' }
    if (-not $vswherePath -and ${env:ProgramFiles(x86)}) {
        $candidate = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { $vswherePath = $candidate }
    }
    if ($vswherePath) {
        $installations = & $vswherePath -all -products '*' -property installationPath
        if ($LASTEXITCODE -ne 0) { throw 'Visual-Studio-Installationen konnten nicht ermittelt werden.' }
        $roots += @($installations | Where-Object { $_ -and (Test-Path -LiteralPath $_ -PathType Container) })
    }
    $roots | Select-Object -Unique
}

function Find-GridfinityCmake {
    $command = Get-Command cmake.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($command) { return $command.Source }
    foreach ($installation in Get-GridfinityVisualStudioRoots) {
        $candidate = Join-Path $installation 'Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe'
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
    }
    throw 'CMake fehlt. CMake zum PATH hinzufügen oder die C++-CMake-Tools in Visual Studio installieren.'
}

function Assert-GridfinityX64Pe {
    param([Parameter(Mandatory = $true)][string]$Path)
    $stream = [System.IO.File]::OpenRead($Path)
    $reader = New-Object System.IO.BinaryReader($stream)
    try {
        if ($stream.Length -lt 64 -or $reader.ReadUInt16() -ne 0x5A4D) {
            throw "Keine Windows-PE-Datei: $Path"
        }
        $stream.Position = 0x3C
        $peOffset = $reader.ReadInt32()
        if ($peOffset -lt 64 -or $peOffset -gt $stream.Length - 26) {
            throw "Ungültiger PE-Header: $Path"
        }
        $stream.Position = $peOffset
        if ($reader.ReadUInt32() -ne 0x00004550 -or $reader.ReadUInt16() -ne 0x8664) {
            throw "Das Paket benötigt Windows-x64-Dateien (AMD64): $Path"
        }
        $stream.Position = $peOffset + 24
        if ($reader.ReadUInt16() -ne 0x20B) { throw "Kein 64-Bit-PE-Format: $Path" }
    } finally {
        $reader.Dispose()
        $stream.Dispose()
    }
}

function Resolve-GridfinityCrtDirectory {
    param([string]$CrtDir = '')
    if ($CrtDir) {
        $resolved = (Resolve-Path -LiteralPath $CrtDir -ErrorAction Stop).ProviderPath
    } else {
        $redistRoots = @()
        if ($env:VCToolsRedistDir -and (Test-Path -LiteralPath $env:VCToolsRedistDir -PathType Container)) {
            $redistRoots += $env:VCToolsRedistDir
        }
        foreach ($installation in Get-GridfinityVisualStudioRoots) {
            $base = Join-Path $installation 'VC\Redist\MSVC'
            if (Test-Path -LiteralPath $base -PathType Container) {
                $redistRoots += @(Get-ChildItem -LiteralPath $base -Directory | Select-Object -ExpandProperty FullName)
            }
        }
        $candidates = foreach ($redist in ($redistRoots | Select-Object -Unique)) {
            $x64 = Join-Path $redist 'x64'
            if (-not (Test-Path -LiteralPath $x64 -PathType Container)) { continue }
            foreach ($directory in Get-ChildItem -LiteralPath $x64 -Directory -Filter 'Microsoft.VC*.CRT') {
                $runtime = Join-Path $directory.FullName 'vcruntime140.dll'
                if (-not (Test-Path -LiteralPath $runtime -PathType Leaf)) { continue }
                $version = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($runtime)
                [PSCustomObject]@{
                    Path = $directory.FullName
                    Version = [Version]::new($version.FileMajorPart, $version.FileMinorPart, $version.FileBuildPart, $version.FilePrivatePart)
                }
            }
        }
        $latest = $candidates | Sort-Object Version -Descending | Select-Object -First 1
        if (-not $latest) {
            throw 'Keine x64-MSVC-Runtime gefunden. Visual Studio C++ Build Tools installieren oder -CrtDir auf Microsoft.VC*.CRT setzen.'
        }
        $resolved = $latest.Path
    }
    foreach ($required in @('vcruntime140.dll', 'msvcp140.dll')) {
        if (-not (Test-Path -LiteralPath (Join-Path $resolved $required) -PathType Leaf)) {
            throw "Die MSVC-Runtime ist unvollständig ($required fehlt): $resolved"
        }
    }
    foreach ($dll in Get-ChildItem -LiteralPath $resolved -File -Filter '*.dll') {
        Assert-GridfinityX64Pe -Path $dll.FullName
    }
    return $resolved
}
