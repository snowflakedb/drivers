# Run ODBC tests on Windows
# Required env vars: DRIVER_PATH, PARAMETER_PATH, DRIVER_TYPE
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location (Join-Path $ScriptDir "..\..\odbc_tests")

try {
    $NPROC = (Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors

    # Use Ninja when both the Ninja build tool and the MSVC compiler are on PATH.
    # Ninja parallelizes at the file level (vs MSBuild's project-level) and has faster
    # startup, but it requires vcvarsall.bat to have been sourced first so that cl.exe
    # is on PATH. Callers that don't set up MSVC env get the default Visual Studio
    # generator (MSBuild), preserving existing behavior.
    $useNinja = (Get-Command ninja.exe -ErrorAction SilentlyContinue) `
           -and (Get-Command cl.exe -ErrorAction SilentlyContinue)
    $desiredGenerator = if ($useNinja) { "Ninja" } else { "Visual Studio" }

    # CMake refuses to reconfigure a build tree with a different generator than it was
    # first created with ("Error: generator ... does not match the generator used previously").
    # In CI the workspace is always fresh so this never triggers; this is defense-in-depth
    # for local devs re-running the script after switching MSVC env state. Wipe cmake-build
    # if the previously-used generator doesn't match the one we're about to use.
    $cacheFile = "cmake-build\CMakeCache.txt"
    if (Test-Path $cacheFile) {
        $existingGenerator = (Select-String -Path $cacheFile -Pattern '^CMAKE_GENERATOR:INTERNAL=(.*)$' |
            ForEach-Object { $_.Matches[0].Groups[1].Value.Trim() } |
            Select-Object -First 1)
        if ($existingGenerator -and -not $existingGenerator.StartsWith($desiredGenerator)) {
            Write-Host "run_tests: generator changed ('$existingGenerator' -> '$desiredGenerator'); wiping cmake-build/"
            Remove-Item -Recurse -Force cmake-build
        }
    }

    New-Item -ItemType Directory -Force -Path cmake-build | Out-Null
    $cmakeArgs = @("-B", "cmake-build", "-D", "DRIVER_TYPE=$env:DRIVER_TYPE")
    if ($useNinja) {
        $cmakeArgs += @("-G", "Ninja", "-DCMAKE_BUILD_TYPE=Debug")
        Write-Host "run_tests: using Ninja generator"
    } else {
        Write-Host "run_tests: using default generator (MSBuild)"
    }
    if (Get-Command ccache -ErrorAction SilentlyContinue) {
        $cmakeArgs += @("-DCMAKE_CXX_COMPILER_LAUNCHER=ccache", "-DCMAKE_C_COMPILER_LAUNCHER=ccache")
        # Use /Z7 (embedded debug info) instead of /Zi (shared PDB). /Zi produces a
        # shared .pdb that serializes parallel writes — ccache marks all /Zi calls as
        # uncacheable because the PDB is a side effect it cannot track. /Z7 embeds
        # debug info in each .obj, making compilations independent and cacheable.
        $cmakeArgs += "-DCMAKE_MSVC_DEBUG_INFORMATION_FORMAT=Embedded"
    }
    if ($env:VCPKG_INSTALLATION_ROOT) {
        $cmakeArgs += "-DCMAKE_TOOLCHAIN_FILE=$env:VCPKG_INSTALLATION_ROOT/scripts/buildsystems/vcpkg.cmake"
    }
    if (Get-Command lld-link.exe -ErrorAction SilentlyContinue) {
        $cmakeArgs += "-DCMAKE_LINKER_TYPE=LLD"
        Write-Host "run_tests: using lld-link"
    }
    cmake @cmakeArgs .

    if (Get-Command ccache -ErrorAction SilentlyContinue) {
        # Write ccache.conf to force depend_mode (env var CCACHE_DEPEND doesn't work on 4.13.6)
        $ccacheConf = Join-Path $env:CCACHE_DIR "ccache.conf"
        Set-Content -Path $ccacheConf -Value "depend_mode = true"
        Write-Host "run_tests: wrote depend_mode=true to $ccacheConf"
        Write-Host "=== ccache directory contents ==="
        Get-ChildItem $env:CCACHE_DIR -Recurse | Measure-Object | Select-Object -ExpandProperty Count | ForEach-Object { Write-Host "  Total files in .ccache: $_" }
        $cacheFiles = Get-ChildItem (Join-Path $env:CCACHE_DIR "??") -ErrorAction SilentlyContinue
        if ($cacheFiles) {
            Write-Host "  Cache subdirectories with content: $($cacheFiles.Count)"
        } else {
            Write-Host "  Cache subdirectories: NONE (empty cache)"
        }
        Write-Host "=== ccache stats BEFORE build ==="
        ccache --show-stats
        Write-Host "=== ccache config (selected) ==="
        ccache --show-config | Select-String "depend_mode|sloppiness|compiler_check|hash_dir"
    }

    if ($useNinja) {
        cmake --build cmake-build --parallel ($NPROC)
    } else {
        cmake --build cmake-build --config Debug --parallel ($NPROC)
    }

    if (Get-Command ccache -ErrorAction SilentlyContinue) {
        Write-Host "=== ccache stats AFTER build ==="
        ccache --show-stats
    }

    # --- Schema lifecycle: pre-create a shared schema for all test processes ---
    $schemaTool = Join-Path $pwd "cmake-build\tools\schema_tool.exe"
    try {
        $schemaName = & $schemaTool create 2>$null
        if ($LASTEXITCODE -eq 0 -and $schemaName) {
            $trimmed = $schemaName.Trim()
            if ($trimmed -match '^TEMP_TEST_SCHEMA_[0-9]+$') {
                $env:ODBC_TEST_SCHEMA = $trimmed
                Write-Host "run_tests: using shared schema $($env:ODBC_TEST_SCHEMA)"
            } else {
                Write-Host "run_tests: schema_tool returned invalid name, falling back to per-process"
            }
        }
    } catch {
        Write-Host "run_tests: schema pre-creation failed, falling back to per-process"
    }

    # Ninja is single-config so `-C Debug` is not needed (and not recognized by ctest
    # for single-config builds). MSBuild is multi-config and needs it.
    if ($useNinja) {
        $ctestArgs = @("-j", ($NPROC * 4), "--test-dir", "cmake-build", "--output-on-failure")
    } else {
        $ctestArgs = @("-j", ($NPROC * 4), "-C", "Debug", "--test-dir", "cmake-build", "--output-on-failure")
    }
    if ($env:CTEST_FILTER) {
        $ctestArgs += @("-R", $env:CTEST_FILTER)
    }
    $ctestArgs += $args
    ctest @ctestArgs
    $ctestExit = $LASTEXITCODE
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
if ($ctestExit -ne 0) { exit $ctestExit }
