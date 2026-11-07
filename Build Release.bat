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
echo Building the optimized release version of ReDox.
echo The first release build may take several minutes.
"%REDOX_CARGO%" build --release --locked --bin redox --target-dir "%~dp0target"
if errorlevel 1 goto failed
if not exist "%~dp0target\release\redox.exe" goto failed
echo.
echo Release ready: "%~dp0target\release\redox.exe"
if /i not "%~1"=="--no-pause" pause
exit /b 0
:missing
echo Rust is not installed or cargo.exe could not be found.
:failed
echo Release build failed. Copy the error above when reporting the problem.
if /i not "%~1"=="--no-pause" pause
exit /b 1
