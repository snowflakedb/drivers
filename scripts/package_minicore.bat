@echo off
REM Exit on any error
setlocal enabledelayedexpansion

call ./scripts/version.bat

echo === Platform: %PLATFORM% ===
echo === Commit SHA: %COMMIT_SHA% ===

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
cargo build --release --package sf_mini_core
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%

echo === Building static library version ===
cargo build --release --package sf_mini_core_static
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%

REM Copy build artifacts
echo === Copying build artifacts ===
REM Copy static library
copy target\release\sf_mini_core_static.lib "%BUILD_DIR%\" >nul
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%
REM Copy dynamic library
copy target\release\sf_mini_core.dll "%BUILD_DIR%\" >nul
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

