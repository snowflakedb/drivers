<#
.SYNOPSIS
    Installs and removes the ODBC MSI and checks which driver name is unregistered.

.DESCRIPTION
    Covers a custom name, the default name, an uninstall command that passes a
    different name, and an upgrade that either keeps the saved name or replaces
    it. The x86 package is checked in the 32-bit registry view.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$MsiPath,

    [Parameter(Mandatory)]
    [string]$UpgradeMsiPath,

    [Parameter(Mandatory)]
    [ValidateSet('x64', 'x86', 'arm64')]
    [string]$Arch,

    [string]$LogDir = 'build\msi-driver-name-logs',

    [switch]$Force
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$MsiPath = (Resolve-Path -LiteralPath $MsiPath).Path
$UpgradeMsiPath = (Resolve-Path -LiteralPath $UpgradeMsiPath).Path

$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'This test installs a per-machine MSI and must run elevated.'
}

New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
$LogDir = (Resolve-Path -LiteralPath $LogDir).Path
$suffix = [guid]::NewGuid().ToString('N').Substring(0, 8)
$script:names = New-Object System.Collections.Generic.List[string]

function Get-OdbcInstRoot {
    if ($Arch -eq 'x86') { return 'HKLM:\SOFTWARE\WOW6432Node\ODBC\ODBCINST.INI' }
    return 'HKLM:\SOFTWARE\ODBC\ODBCINST.INI'
}

function Get-SavedNameKey {
    # A 32-bit package writes HKLM\Software\Snowflake\ODBCDriver\x86, which
    # Windows stores at HKLM\SOFTWARE\WOW6432Node\Snowflake\ODBCDriver\x86.
    if ($Arch -eq 'x86') {
        return 'HKLM:\SOFTWARE\WOW6432Node\Snowflake\ODBCDriver\x86'
    }
    return "HKLM:\SOFTWARE\Snowflake\ODBCDriver\$Arch"
}

function Get-SentinelKey {
    return Join-Path (Get-OdbcInstRoot) 'Snowflake ODBC'
}

function Get-OdbcDriversKey {
    return Join-Path (Get-OdbcInstRoot) 'ODBC Drivers'
}

function Invoke-Msi {
    param(
        [Parameter(Mandatory)]
        [ValidateSet('install', 'remove')]
        [string]$Action,
        [Parameter(Mandatory)]
        [string]$Path,
        [Parameter(Mandatory)]
        [string]$LogName,
        [string]$DriverName,
        [switch]$AllowMissing
    )
    $log = Join-Path $LogDir $LogName
    $argList = New-Object System.Collections.Generic.List[string]
    if ($Action -eq 'install') { $argList.Add('/i') } else { $argList.Add('/x') }
    $argList.Add($Path)
    $argList.Add('/qn')
    $argList.Add('/l*v')
    $argList.Add($log)
    if ($PSBoundParameters.ContainsKey('DriverName')) {
        $argList.Add("DRIVER_NAME=$DriverName")
    }
    $proc = Start-Process -FilePath "$env:SystemRoot\System32\msiexec.exe" -ArgumentList $argList.ToArray() -Wait -PassThru
    $code = $proc.ExitCode
    if ($code -eq 0 -or $code -eq 3010) { return }
    if ($AllowMissing -and $code -eq 1605) { return }
    throw "msiexec $Action exited $code. Log: $log"
}

function Assert-DriverRegistered {
    param([Parameter(Mandatory)][string]$Name, [Parameter(Mandatory)][string]$Because)
    $root = Get-OdbcInstRoot
    $sub = Join-Path $root $Name
    $drivers = Join-Path $root 'ODBC Drivers'
    if (-not (Test-Path -LiteralPath $sub)) { throw "missing $sub ($Because)" }
    if (-not (Test-Path -LiteralPath $drivers)) { throw "missing $drivers ($Because)" }
    $installed = (Get-ItemProperty -LiteralPath $drivers).PSObject.Properties[$Name]
    if ($null -eq $installed) { throw "ODBC Drivers value '$Name' is missing ($Because)" }
    if ($installed.Value -ne 'Installed') { throw "ODBC Drivers value '$Name' is '$($installed.Value)' ($Because)" }
}

function Assert-DriverAbsent {
    param([Parameter(Mandatory)][string]$Name, [Parameter(Mandatory)][string]$Because)
    $root = Get-OdbcInstRoot
    $sub = Join-Path $root $Name
    if (Test-Path -LiteralPath $sub) { throw "key still present: $sub ($Because)" }
    $drivers = Join-Path $root 'ODBC Drivers'
    if (Test-Path -LiteralPath $drivers) {
        $props = Get-ItemProperty -LiteralPath $drivers
        if ($props.PSObject.Properties.Name -contains $Name) {
            throw "ODBC Drivers value '$Name' still present ($Because)"
        }
    }
}

function Assert-SavedName {
    param([Parameter(Mandatory)][string]$Expected, [Parameter(Mandatory)][string]$Because)
    $key = Get-SavedNameKey
    if (-not (Test-Path -LiteralPath $key)) { throw "missing $key ($Because)" }
    $actual = (Get-ItemProperty -LiteralPath $key).PSObject.Properties['DriverName']
    if ($null -eq $actual) { throw "saved DriverName is missing ($Because)" }
    if ($actual.Value -ne $Expected) { throw "saved DriverName is '$($actual.Value)', expected '$Expected' ($Because)" }
}

function Assert-SavedNameAbsent {
    param([Parameter(Mandatory)][string]$Because)
    $key = Get-SavedNameKey
    if (-not (Test-Path -LiteralPath $key)) { return }
    $props = Get-ItemProperty -LiteralPath $key
    if ($props.PSObject.Properties.Name -contains 'DriverName') {
        throw "saved DriverName '$($props.DriverName)' still present ($Because)"
    }
}

function Set-Sentinel {
    $key = Get-SentinelKey
    New-Item -Path $key -Force | Out-Null
    New-ItemProperty -LiteralPath $key -Name Marker -Value keepme -PropertyType String -Force | Out-Null
    $drivers = Get-OdbcDriversKey
    if (-not (Test-Path -LiteralPath $drivers)) {
        New-Item -Path $drivers -Force | Out-Null
    }
    New-ItemProperty -LiteralPath $drivers -Name 'Snowflake ODBC' -Value Installed -PropertyType String -Force | Out-Null
}

function Assert-Sentinel {
    param([Parameter(Mandatory)][string]$Because)
    $key = Get-SentinelKey
    if (-not (Test-Path -LiteralPath $key)) { throw "sentinel key missing ($Because)" }
    $marker = (Get-ItemProperty -LiteralPath $key).PSObject.Properties['Marker']
    if ($null -eq $marker) { throw "sentinel Marker is missing ($Because)" }
    if ($marker.Value -ne 'keepme') { throw "sentinel Marker is '$($marker.Value)' ($Because)" }
    $drivers = Get-OdbcDriversKey
    if (-not (Test-Path -LiteralPath $drivers)) { throw "ODBC Drivers key missing ($Because)" }
    $installed = (Get-ItemProperty -LiteralPath $drivers).PSObject.Properties['Snowflake ODBC']
    if ($null -eq $installed) { throw "ODBC Drivers value 'Snowflake ODBC' is missing ($Because)" }
    if ($installed.Value -ne 'Installed') {
        throw "ODBC Drivers value 'Snowflake ODBC' is '$($installed.Value)' ($Because)"
    }
}

function Remove-Sentinel {
    $key = Get-SentinelKey
    if (Test-Path -LiteralPath $key) {
        Remove-Item -LiteralPath $key -Recurse -Force
    }
    $drivers = Get-OdbcDriversKey
    if ((Test-Path -LiteralPath $drivers) -and
        ((Get-ItemProperty -LiteralPath $drivers).PSObject.Properties.Name -contains 'Snowflake ODBC')) {
        Remove-ItemProperty -LiteralPath $drivers -Name 'Snowflake ODBC' -Force
    }
}

function New-DriverName {
    param([Parameter(Mandatory)][string]$Label)
    $name = "CI$Label$suffix"
    $script:names.Add($name)
    return $name
}

function Remove-TestRegistrations {
    $root = Get-OdbcInstRoot
    $drivers = Join-Path $root 'ODBC Drivers'
    foreach ($name in $script:names) {
        $sub = Join-Path $root $name
        if (Test-Path -LiteralPath $sub) { Remove-Item -LiteralPath $sub -Recurse -Force }
        if ((Test-Path -LiteralPath $drivers) -and
            ((Get-ItemProperty -LiteralPath $drivers).PSObject.Properties.Name -contains $name)) {
            Remove-ItemProperty -LiteralPath $drivers -Name $name -Force
        }
    }
    $saved = Get-SavedNameKey
    if (Test-Path -LiteralPath $saved) { Remove-Item -LiteralPath $saved -Recurse -Force }
    Remove-Sentinel
}

if (-not $Force) {
    foreach ($key in @((Get-SentinelKey), (Get-SavedNameKey))) {
        if (Test-Path -LiteralPath $key) {
            throw "Registry key already exists: $key. Re-run with -Force only on a machine whose Snowflake ODBC registration can be replaced."
        }
    }
    $drivers = Get-OdbcDriversKey
    if ((Test-Path -LiteralPath $drivers) -and
        ((Get-ItemProperty -LiteralPath $drivers).PSObject.Properties.Name -contains 'Snowflake ODBC')) {
        throw "ODBC Drivers already lists 'Snowflake ODBC'. Re-run with -Force only on a machine whose Snowflake ODBC registration can be replaced."
    }
}

try {
    $custom = New-DriverName 'Custom'
    Set-Sentinel
    Invoke-Msi -Action install -Path $MsiPath -LogName 'custom-install.log' -DriverName $custom
    Assert-DriverRegistered -Name $custom -Because 'custom install'
    Assert-SavedName -Expected $custom -Because 'custom install'
    Assert-Sentinel -Because 'custom install'
    Invoke-Msi -Action remove -Path $MsiPath -LogName 'custom-remove.log'
    Assert-DriverAbsent -Name $custom -Because 'custom uninstall drops the saved name'
    Assert-SavedNameAbsent -Because 'custom uninstall'
    Assert-Sentinel -Because 'custom uninstall leaves a pre-existing Snowflake ODBC key'

    Remove-Sentinel
    Invoke-Msi -Action install -Path $MsiPath -LogName 'default-install.log'
    Assert-DriverRegistered -Name 'Snowflake ODBC' -Because 'default install'
    Assert-SavedName -Expected 'Snowflake ODBC' -Because 'default install'
    Invoke-Msi -Action remove -Path $MsiPath -LogName 'default-remove.log'
    Assert-DriverAbsent -Name 'Snowflake ODBC' -Because 'default uninstall'
    Assert-SavedNameAbsent -Because 'default uninstall'

    $installed = New-DriverName 'Installed'
    $other = New-DriverName 'Other'
    Set-Sentinel
    Invoke-Msi -Action install -Path $MsiPath -LogName 'mismatch-install.log' -DriverName $installed
    Invoke-Msi -Action remove -Path $MsiPath -LogName 'mismatch-remove.log' -DriverName $other
    Assert-DriverAbsent -Name $installed -Because 'uninstall ignores a different DRIVER_NAME'
    Assert-DriverAbsent -Name $other -Because 'the name passed only at uninstall is not registered'
    Assert-SavedNameAbsent -Because 'mismatch uninstall'
    Assert-Sentinel -Because 'mismatch uninstall'

    # The 0.0.1 package is built from this installer, so these upgrades cover a
    # product that already saved DriverName.
    $kept = New-DriverName 'Kept'
    Invoke-Msi -Action install -Path $UpgradeMsiPath -LogName 'inherit-old-install.log' -DriverName $kept
    Invoke-Msi -Action install -Path $MsiPath -LogName 'inherit-upgrade.log'
    Assert-DriverRegistered -Name $kept -Because 'upgrade with no new name keeps the saved name'
    Assert-SavedName -Expected $kept -Because 'upgrade with no new name'
    Assert-Sentinel -Because 'inherit upgrade'
    Invoke-Msi -Action remove -Path $MsiPath -LogName 'inherit-remove.log'
    Assert-DriverAbsent -Name $kept -Because 'removal after inherit upgrade'
    Assert-SavedNameAbsent -Because 'removal after inherit upgrade'

    $oldName = New-DriverName 'Old'
    $newName = New-DriverName 'New'
    Invoke-Msi -Action install -Path $UpgradeMsiPath -LogName 'rename-old-install.log' -DriverName $oldName
    Invoke-Msi -Action install -Path $MsiPath -LogName 'rename-upgrade.log' -DriverName $newName
    Assert-DriverAbsent -Name $oldName -Because 'upgrade with a new name removes the old registration'
    Assert-DriverRegistered -Name $newName -Because 'upgrade with a new name'
    Assert-SavedName -Expected $newName -Because 'upgrade with a new name'
    Assert-Sentinel -Because 'rename upgrade'
    Invoke-Msi -Action remove -Path $MsiPath -LogName 'rename-remove.log'
    Assert-DriverAbsent -Name $oldName -Because 'final uninstall'
    Assert-DriverAbsent -Name $newName -Because 'final uninstall'
    Assert-SavedNameAbsent -Because 'final uninstall'
    Assert-Sentinel -Because 'final uninstall'
}
finally {
    foreach ($pair in @(
            @{ Path = $MsiPath; Log = 'cleanup-current.log' },
            @{ Path = $UpgradeMsiPath; Log = 'cleanup-upgrade.log' }
        )) {
        try {
            Invoke-Msi -Action remove -Path $pair.Path -LogName $pair.Log -AllowMissing
        }
        catch {
            Write-Warning $_
        }
    }
    Remove-TestRegistrations
}
