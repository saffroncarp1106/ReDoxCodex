@echo off
setlocal
cd /d "%~dp0"
set "CODEX_RUST_CARGO=%USERPROFILE%\.cargo\bin\cargo.exe"
if exist "%CODEX_RUST_CARGO%" goto build
where cargo >nul 2>nul
if errorlevel 1 goto missing
set "CODEX_RUST_CARGO=cargo"
:build
echo Building Codex Rust. The window will open when the build finishes.
"%CODEX_RUST_CARGO%" build --locked
if errorlevel 1 goto failed
start "" "%~dp0target\debug\codex-rust.exe"
exit /b 0
:missing
echo Rust is not installed or cargo.exe could not be found.
:failed
echo Launch failed. Copy the error above when reporting the problem.
pause
exit /b 1
