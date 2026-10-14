@echo off
cd /d "%~dp0"
if exist "GridfinityStudioDesign2.exe" (
    start "" "%~dp0GridfinityStudioDesign2.exe"
) else (
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\build.ps1
    if exist "GridfinityStudioDesign2.exe" start "" "%~dp0GridfinityStudioDesign2.exe"
)
