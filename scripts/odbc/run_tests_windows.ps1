# Run ODBC tests on Windows
# Required env vars: DRIVER_PATH, PARAMETER_PATH, DRIVER_TYPE
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location (Join-Path $ScriptDir "..\..\odbc_tests")

try {
    $NPROC = (Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors
    
    New-Item -ItemType Directory -Force -Path cmake-build | Out-Null
    $cmakeArgs = @("-B", "cmake-build", "-D", "DRIVER_TYPE=$env:DRIVER_TYPE")
    if ($env:VCPKG_INSTALLATION_ROOT) {
        $cmakeArgs += "-DCMAKE_TOOLCHAIN_FILE=$env:VCPKG_INSTALLATION_ROOT/scripts/buildsystems/vcpkg.cmake"
    }
    cmake @cmakeArgs .
    cmake --build cmake-build --config Debug --parallel ($NPROC * 2)
    $ctestArgs = @("-j", ($NPROC * 4), "-C", "Debug", "--test-dir", "cmake-build", "--output-on-failure")
    if ($env:CTEST_FILTER) {
        $ctestArgs += @("-R", $env:CTEST_FILTER)
    }
    $ctestArgs += $args
    ctest @ctestArgs
}
finally {
    Pop-Location
}
