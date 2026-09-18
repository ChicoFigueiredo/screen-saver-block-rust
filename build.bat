@echo off
setlocal

rem Compila o aplicativo na plataforma atual.
rem Uso: build.bat [debug^|release^|check^|test]

set "MODE=%~1"
if "%MODE%"=="" set "MODE=release"

if /I "%MODE%"=="release" (
  cargo build --release
  if errorlevel 1 exit /b %errorlevel%
  echo Binario gerado em: target\release\block-screen-saver.exe
  exit /b 0
)

if /I "%MODE%"=="debug" (
  cargo build
  if errorlevel 1 exit /b %errorlevel%
  echo Binario gerado em: target\debug\block-screen-saver.exe
  exit /b 0
)

if /I "%MODE%"=="check" (
  cargo check
  exit /b %errorlevel%
)

if /I "%MODE%"=="test" (
  cargo test
  exit /b %errorlevel%
)

echo Uso: build.bat [debug^|release^|check^|test]
exit /b 2
