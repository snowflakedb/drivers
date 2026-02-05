# Run ODBC tests on Windows
# Required env vars: DRIVER_PATH, PARAMETER_PATH, DRIVER_TYPE
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location (Join-Path $ScriptDir "..\..\odbc_tests")

try {
    $NPROC = (Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors
    
    New-Item -ItemType Directory -Force -Path cmake-build | Out-Null
    cmake -B cmake-build -D DRIVER_TYPE=$env:DRIVER_TYPE .
    cmake --build cmake-build --config Debug --parallel $NPROC
    ctest -j $NPROC -C Debug --test-dir cmake-build --output-on-failure
}
finally {
    Pop-Location
}
