<#
.SYNOPSIS
    Installs the standard x64 MSI and then the same-version FIPS MSI.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$StandardMsi,

    [Parameter(Mandatory)]
    [string]$FipsMsi,

    [string]$LogDir = 'build\msi-fips-replace-logs'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$StandardMsi = (Resolve-Path -LiteralPath $StandardMsi).Path
$FipsMsi = (Resolve-Path -LiteralPath $FipsMsi).Path
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'This test installs a per-machine MSI and must run elevated.'
}

New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
$LogDir = (Resolve-Path -LiteralPath $LogDir).Path

function Invoke-QuietInstall {
    param(
        [Parameter(Mandatory)][ValidateSet('install', 'remove')][string]$Action,
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$LogName
    )
    $log = Join-Path $LogDir $LogName
    $flag = if ($Action -eq 'install') { '/i' } else { '/x' }
    $proc = Start-Process -FilePath "$env:SystemRoot\System32\msiexec.exe" -ArgumentList @(
        $flag, $Path, '/qn', '/l*v', $log
    ) -Wait -PassThru
    if ($proc.ExitCode -eq 0 -or $proc.ExitCode -eq 3010) { return }
    if ($Action -eq 'remove' -and $proc.ExitCode -eq 1605) { return }
    throw "msiexec $Action exited $($proc.ExitCode) for $Path. Log: $log"
}

function Get-SnowflakeOdbcProducts {
    $roots = @(
        'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
        'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall'
    )
    $found = New-Object System.Collections.Generic.List[string]
    foreach ($root in $roots) {
        if (-not (Test-Path -LiteralPath $root)) { continue }
        foreach ($key in Get-ChildItem -LiteralPath $root) {
            $display = (Get-ItemProperty -LiteralPath $key.PSPath).PSObject.Properties['DisplayName']
            if ($null -ne $display -and $display.Value -like 'Snowflake ODBC Driver*') {
                $found.Add([string]$display.Value)
            }
        }
    }
    return @($found)
}

try {
    Invoke-QuietInstall -Action install -Path $StandardMsi -LogName 'standard-install.log'
    Invoke-QuietInstall -Action install -Path $FipsMsi -LogName 'fips-upgrade.log'
    $products = @(Get-SnowflakeOdbcProducts)
    if ($products.Count -ne 1) {
        throw "expected one Snowflake ODBC product after the same-version upgrade, found $($products.Count): $($products -join '; ')"
    }
    if ($products[0] -notlike '*FIPS*') {
        throw "the remaining product is '$($products[0])'"
    }
}
finally {
    Invoke-QuietInstall -Action remove -Path $FipsMsi -LogName 'fips-remove.log'
    Invoke-QuietInstall -Action remove -Path $StandardMsi -LogName 'standard-remove.log'
}

Write-Host 'Same-version FIPS MSI replaced the standard x64 MSI'
