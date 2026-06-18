@echo off
REM Exit on any error
setlocal enabledelayedexpansion

call ./scripts/version.bat

echo === Platform: %PLATFORM% ===
echo === Commit SHA: %COMMIT_SHA% ===
REM Set platform target based on PLATFORM
if "%PLATFORM%"=="windows-x86_64" (
    set PLATFORM_TARGET=x86_64-pc-windows-msvc
) else if "%PLATFORM%"=="windows-i686" (
    set PLATFORM_TARGET=i686-pc-windows-msvc
) else if "%PLATFORM%"=="windows-aarch64" (
    set PLATFORM_TARGET=aarch64-pc-windows-msvc
) else (
    echo Unknown platform: %PLATFORM%
    exit /b 1
)

echo === Platform target: %PLATFORM_TARGET% ===

echo === Ensuring target is installed ===
rustup target add %PLATFORM_TARGET%
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%


echo === Checking if cbindgen is installed ===
REM Install cbindgen if not available
cbindgen --version >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo === Installing cbindgen ===
    cargo install cbindgen
)

REM Set build directory
set BUILD_DIR=%cd%\build
echo === Using build directory: %BUILD_DIR% ===

REM Create build directory if it doesn't exist
echo === Creating build directory: %BUILD_DIR% ===
if not exist "%BUILD_DIR%" mkdir "%BUILD_DIR%"
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%

REM Generate C header file
echo === Generating C header file: %BUILD_DIR%\sf_mini_core.h ===
cbindgen --config sf_mini_core\cbindgen.toml --crate sf_mini_core > "%BUILD_DIR%\sf_mini_core.h"
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%

REM Build release version
echo === Building dynamic library version ===
cargo build --release --package sf_mini_core --target %PLATFORM_TARGET%
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%

echo === Building static library version ===
cargo build --release --package sf_mini_core_static --target %PLATFORM_TARGET%
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%

REM Copy build artifacts
echo === Copying build artifacts ===
REM Copy static library
copy target\%PLATFORM_TARGET%\release\sf_mini_core_static.lib "%BUILD_DIR%\" >nul
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%
REM Copy dynamic library
copy target\%PLATFORM_TARGET%\release\sf_mini_core.dll "%BUILD_DIR%\" >nul
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%

set PACKAGE_NAME=sf_mini_core_%PLATFORM%_%VERSION%_SNAPSHOT_%COMMIT_SHA%.tar.gz

REM Create archive
echo === Creating archive: %PACKAGE_NAME% ===
pushd "%BUILD_DIR%"
tar -czf %PACKAGE_NAME% sf_mini_core.h sf_mini_core_static.lib sf_mini_core.dll
if %ERRORLEVEL% neq 0 (
    popd
    exit /b %ERRORLEVEL%
)
popd

echo === Successfully created archive at %BUILD_DIR%\%PACKAGE_NAME% ===

endlocal

