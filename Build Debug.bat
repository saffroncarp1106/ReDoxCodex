@echo off
setlocal
cd /d "%~dp0"
if errorlevel 1 goto failed
set "REDOX_CARGO=%USERPROFILE%\.cargo\bin\cargo.exe"
if exist "%REDOX_CARGO%" goto build
where cargo.exe >nul 2>nul
if errorlevel 1 goto missing
set "REDOX_CARGO=cargo.exe"
:build
echo Building the debug version of ReDox.
"%REDOX_CARGO%" build --locked --bin redox --target-dir "%~dp0target"
if errorlevel 1 goto failed
if not exist "%~dp0target\debug\redox.exe" goto failed
echo.
echo Debug ready: "%~dp0target\debug\redox.exe"
if /i not "%~1"=="--no-pause" pause
exit /b 0
:missing
echo Rust is not installed or cargo.exe could not be found.
:failed
echo Debug build failed. Copy the error above when reporting the problem.
if /i not "%~1"=="--no-pause" pause
exit /b 1
