# Build and run ODBC tests locally on Windows
# Requires CMake to be available in PATH
# Passes any extra arguments through to ctest
$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $ProjectRoot

$env:PARAMETER_PATH = Join-Path $ProjectRoot "parameters.json"
$vcpkgRoot = if ($env:VCPKG_ROOT) { $env:VCPKG_ROOT } elseif ($env:VCPKG_INSTALLATION_ROOT) { $env:VCPKG_INSTALLATION_ROOT } else { $null }
if ($vcpkgRoot) {
    $env:OPENSSL_DIR = Join-Path $vcpkgRoot "installed\x64-windows"
    $env:OPENSSL_LIB_DIR = Join-Path $vcpkgRoot "installed\x64-windows\lib"
    $env:OPENSSL_INCLUDE_DIR = Join-Path $vcpkgRoot "installed\x64-windows\include"
    $vcpkgBinDir = Join-Path $vcpkgRoot "installed\x64-windows\bin"
    if ($env:PATH -notlike "*$vcpkgBinDir*") {
        $env:PATH = "$vcpkgBinDir;$env:PATH"
    }
}

try {
    cargo build
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
}
finally {
    Pop-Location
}

$env:DRIVER_PATH = Join-Path $ProjectRoot "target\debug\sfodbc.dll"

Push-Location $PSScriptRoot

try {
    $NPROC = (Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors

    New-Item -ItemType Directory -Force -Path cmake-build | Out-Null

    $cmakeArgs = @("-B", "cmake-build", "-D", "DRIVER_TYPE=NEW")
    $vcpkgRoot = if ($env:VCPKG_INSTALLATION_ROOT) { $env:VCPKG_INSTALLATION_ROOT } elseif ($env:VCPKG_ROOT) { $env:VCPKG_ROOT } else { $null }
    if ($vcpkgRoot) {
        $cmakeArgs += "-DCMAKE_TOOLCHAIN_FILE=$vcpkgRoot/scripts/buildsystems/vcpkg.cmake"
    }
    Write-Host "`n=== CMake Configuration ===" -ForegroundColor Cyan
    Write-Host "Working directory: $(Get-Location)" -ForegroundColor Yellow
    Write-Host "cmake $($cmakeArgs -join ' ') ." -ForegroundColor Yellow
    Write-Host "`n=== Environment Variables ===" -ForegroundColor Cyan
    $envPairs = @("PARAMETER_PATH", "DRIVER_PATH", "OPENSSL_DIR", "OPENSSL_LIB_DIR", "OPENSSL_INCLUDE_DIR", "RUST_BACKTRACE") | ForEach-Object {
        $val = [System.Environment]::GetEnvironmentVariable($_)
        if ($val) { "$_=$val" }
    }
    Write-Host ($envPairs -join ";") -ForegroundColor Yellow
    Write-Host "==============================`n" -ForegroundColor Cyan

    & "C:\Program Files\CMake\bin\cmake" @cmakeArgs .
    if ($LASTEXITCODE -ne 0) { throw "cmake configure failed" }

    & "C:\Program Files\CMake\bin\cmake" --build cmake-build --config Debug --parallel $NPROC
    if ($LASTEXITCODE -ne 0) { throw "cmake build failed" }
    $env:RUST_BACKTRACE = "1"

    & "C:\Program Files\CMake\bin\ctest" -j $NPROC -C Debug --test-dir cmake-build --output-on-failure @args
    if ($LASTEXITCODE -ne 0) { throw "ctest failed" }
}
finally {
    Pop-Location
}
