# Run ODBC tests on Windows
# Required env vars: DRIVER_PATH, PARAMETER_PATH, DRIVER_TYPE
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location (Join-Path $ScriptDir "..\..\odbc_tests")

try {
    $NPROC = (Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors

    New-Item -ItemType Directory -Force -Path cmake-build | Out-Null
    $cmakeArgs = @("-B", "cmake-build", "-D", "DRIVER_TYPE=$env:DRIVER_TYPE")
    if (Get-Command ccache -ErrorAction SilentlyContinue) {
        $cmakeArgs += @("-DCMAKE_CXX_COMPILER_LAUNCHER=ccache", "-DCMAKE_C_COMPILER_LAUNCHER=ccache")
    }
    if ($env:VCPKG_INSTALLATION_ROOT) {
        $cmakeArgs += "-DCMAKE_TOOLCHAIN_FILE=$env:VCPKG_INSTALLATION_ROOT/scripts/buildsystems/vcpkg.cmake"
    }
    cmake @cmakeArgs .
    cmake --build cmake-build --config Debug --parallel ($NPROC)

    # --- Schema lifecycle: pre-create a shared schema for all test processes ---
    $schemaTool = Join-Path $pwd "cmake-build\tools\schema_tool.exe"
    try {
        $schemaName = & $schemaTool create 2>$null
        if ($LASTEXITCODE -eq 0 -and $schemaName) {
            $env:ODBC_TEST_SCHEMA = $schemaName.Trim()
            Write-Host "run_tests: using shared schema $($env:ODBC_TEST_SCHEMA)"
        }
    } catch {
        Write-Host "run_tests: schema pre-creation failed, falling back to per-process"
    }

    $ctestArgs = @("-j", ($NPROC * 4), "-C", "Debug", "--test-dir", "cmake-build", "--output-on-failure")
    if ($env:CTEST_FILTER) {
        $ctestArgs += @("-R", $env:CTEST_FILTER)
    }
    $ctestArgs += $args
    ctest @ctestArgs
}
finally {
    if ($env:ODBC_TEST_SCHEMA) {
        try {
            & $schemaTool drop $env:ODBC_TEST_SCHEMA 2>$null
        } catch {}
        $env:ODBC_TEST_SCHEMA = $null
    }
    Pop-Location
}
