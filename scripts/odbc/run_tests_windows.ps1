# Run ODBC tests on Windows
# Required env vars: DRIVER_PATH, PARAMETER_PATH, DRIVER_TYPE
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location (Join-Path $ScriptDir "..\..\odbc_tests")

try {
    $NPROC = (Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors
    # Validate required environment variables
    if (-not $env:DRIVER_PATH) {
        throw "DRIVER_PATH environment variable is required"
    }
    if (-not (Test-Path $env:DRIVER_PATH -PathType Leaf)) {
        throw "DRIVER_PATH '$env:DRIVER_PATH' does not exist or is not a file"
    }

    
    New-Item -ItemType Directory -Force -Path cmake-build | Out-Null
    $cmakeArgs = @("-B", "cmake-build", "-D", "DRIVER_TYPE=$env:DRIVER_TYPE")
    if ($env:VCPKG_INSTALLATION_ROOT) {
        $cmakeArgs += "-DCMAKE_TOOLCHAIN_FILE=$env:VCPKG_INSTALLATION_ROOT/scripts/buildsystems/vcpkg.cmake"
    }
    cmake @cmakeArgs .
    cmake --build cmake-build --config Debug --parallel $NPROC
    ctest -j $NPROC -C Debug --test-dir cmake-build --output-on-failure
}
finally {
    Pop-Location
}
