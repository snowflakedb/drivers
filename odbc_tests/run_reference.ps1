# Build and run ODBC reference tests on Windows
# This is the Windows equivalent of run_reference.sh
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir

$ReferenceOdbcVersion = (Get-Content "$ProjectRoot\ci\reference-odbc-version" -Raw).Trim()

Write-Host "Using reference ODBC driver version: $ReferenceOdbcVersion (Windows x86_64)"

function Get-InstalledSnowflakeOdbc {
    $UninstallRoots = @(
        "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "HKCU:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*"
    )

    foreach ($root in $UninstallRoots) {
        $match = Get-ItemProperty -Path $root -ErrorAction SilentlyContinue |
            Where-Object {
                $_.DisplayName -and
                $_.DisplayName -match "Snowflake" -and
                $_.DisplayName -match "ODBC"
            } |
            Select-Object -First 1
        if ($match) {
            return $match
        }
    }

    return $null
}

function Get-OdbcRegistryDriverInfo {
    $odbcDriverKeys = @(
        "HKLM:\SOFTWARE\ODBC\ODBCINST.INI\SnowflakeDSIIDriver",
        "HKLM:\SOFTWARE\WOW6432Node\ODBC\ODBCINST.INI\SnowflakeDSIIDriver"
    )

    foreach ($key in $odbcDriverKeys) {
        $entry = Get-ItemProperty -Path $key -ErrorAction SilentlyContinue
        if ($entry -and $entry.Driver) {
            return $entry
        }
    }

    return $null
}

$installed = Get-InstalledSnowflakeOdbc
if ($installed) {
    Write-Host "Detected installed Snowflake ODBC driver version: $($installed.DisplayVersion)"
    if ($installed.DisplayVersion -and $installed.DisplayVersion -ne $ReferenceOdbcVersion) {
        Write-Warning "Installed version ($($installed.DisplayVersion)) differs from reference version ($ReferenceOdbcVersion)."
    }
} else {
    Write-Warning "Could not detect Snowflake ODBC driver from uninstall registry entries; trying ODBC driver registry."
}

$odbcRegistryInfo = Get-OdbcRegistryDriverInfo

$DriverCandidates = @()
if ($odbcRegistryInfo -and $odbcRegistryInfo.Driver) {
    $DriverCandidates += $odbcRegistryInfo.Driver
}
if ($installed -and $installed.InstallLocation) {
    $DriverCandidates += (Join-Path $installed.InstallLocation "Bin\SnowflakeDSII.dll")
}
$DriverCandidates += @(
    "C:\Program Files\Snowflake ODBC Driver\Bin\SnowflakeDSII.dll",
    "C:\Program Files (x86)\Snowflake ODBC Driver\Bin\SnowflakeDSII.dll"
)

$DriverPath = $DriverCandidates |
    Select-Object -Unique |
    Where-Object { $_ -and (Test-Path $_) } |
    Select-Object -First 1
if (-not (Test-Path $DriverPath)) {
    throw "Could not find Snowflake ODBC driver DLL. Checked: $($DriverCandidates -join ', ')"
}

$ParameterPath = Join-Path $ProjectRoot "parameters.json"
if (-not (Test-Path $ParameterPath)) {
    throw "parameters.json not found. Please run .\scripts\decode_secrets.sh first."
}

Write-Host "Building and running ODBC reference tests..."

$env:DRIVER_PATH = $DriverPath
$env:PARAMETER_PATH = $ParameterPath
$env:DRIVER_TYPE = "OLD"

& "$ProjectRoot\scripts\odbc\run_tests_windows.ps1" @args

Write-Host "ODBC reference tests completed!"
