@echo off
cd /d "%~dp0"
if exist "GridfinityStudio.exe" (
    start "" "%~dp0GridfinityStudio.exe"
) else (
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\build.ps1
    if exist "GridfinityStudio.exe" start "" "%~dp0GridfinityStudio.exe"
)
