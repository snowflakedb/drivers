# Retry cmake --build for Windows flakes:
# - ARM64 file-in-use (mt.exe / z-applocal / CatchAddTests)
# - MSVC C1002 "compiler is out of heap space in pass 2" on huge /Z7 TUs
#   (PowerQuery navigate_and_load/test.cpp is ~1MB). Retry that case at -j1
#   so the large TU compiles alone. File-in-use keeps the original parallelism.
#
# Must run with cwd = odbc_tests/ (cmake-build is relative).
function Invoke-CmakeBuildWithRetry {
    param(
        [bool]$UseNinja,
        [int]$Parallelism,
        [int]$MaxAttempts = 3,
        [string]$LogName = "odbc-cmake-build.log"
    )
    $logDir = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [System.IO.Path]::GetTempPath() }
    $buildLog = Join-Path $logDir $LogName
    $currentParallelism = $Parallelism
    for ($attempt = 1; $attempt -le $MaxAttempts; $attempt++) {
        Write-Host "cmake --build attempt $attempt/$MaxAttempts (parallel=$currentParallelism)"
        if ($UseNinja) {
            cmake --build cmake-build --parallel $currentParallelism 2>&1 | Tee-Object -FilePath $buildLog
        } else {
            cmake --build cmake-build --config Debug --parallel $currentParallelism 2>&1 | Tee-Object -FilePath $buildLog
        }
        if ($LASTEXITCODE -eq 0) { return }
        $exitCode = $LASTEXITCODE
        $logText = Get-Content $buildLog -Raw
        $heapOom = $logText -match 'C1002' -or $logText -match 'out of heap space'
        $retryable = $heapOom `
            -or $logText -match 'being used by another process' `
            -or $logText -match 'ninja: build stopped'
        if (-not $retryable -or $attempt -eq $MaxAttempts) {
            throw "cmake --build failed with exit code $exitCode (attempt $attempt/$MaxAttempts). See $buildLog"
        }
        if ($heapOom -and $currentParallelism -gt 1) {
            Write-Host "MSVC C1002 heap space; retrying with --parallel 1"
            $currentParallelism = 1
        } else {
            Write-Host "Retryable Windows build failure detected; waiting before retry..."
        }
        Start-Sleep -Seconds (15 * $attempt)
    }
}
