@echo off
setlocal
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
if errorlevel 1 exit /b 1
where cl
where link
cd /d "%~dp0"
rustup run stable-x86_64-pc-windows-msvc cargo build --release
if errorlevel 1 exit /b 1
