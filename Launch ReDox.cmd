@echo off
setlocal
cd /d "%~dp0"
if errorlevel 1 goto failed
set "REDOX_PROFILE=release"
if /i "%~1"=="--debug" (
    set "REDOX_PROFILE=debug"
) else if not "%~1"=="" if /i not "%~1"=="--release" goto usage
if exist "%~dp0redox.exe" (
    start "" /d "%~dp0" "%~dp0redox.exe"
    exit /b 0
)
if exist "%~dp0ReDox-Windows-x64.exe" (
    start "" /d "%~dp0" "%~dp0ReDox-Windows-x64.exe"
    exit /b 0
)
if exist "%~dp0target\%REDOX_PROFILE%\redox.exe" goto launch
if "%REDOX_PROFILE%"=="debug" (
    call "%~dp0Build Debug.bat" --no-pause
) else (
    call "%~dp0Build Release.bat" --no-pause
)
if errorlevel 1 goto failed
:launch
start "" /d "%~dp0" "%~dp0target\%REDOX_PROFILE%\redox.exe"
if errorlevel 1 goto failed
exit /b 0
:usage
echo Usage: "%~nx0" [--debug ^| --release]
exit /b 1
:failed
echo Build or launch failed. Copy the error above when reporting the problem.
pause
exit /b 1
