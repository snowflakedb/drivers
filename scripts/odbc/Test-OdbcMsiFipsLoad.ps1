<#
.SYNOPSIS
    Installs the FIPS MSI and loads sfodbc.dll without its directory on PATH.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$MsiPath,

    [string]$LogDir = 'build\msi-fips-load-logs'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$MsiPath = (Resolve-Path -LiteralPath $MsiPath).Path
$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'This test installs a per-machine MSI and must run elevated.'
}

New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
$LogDir = (Resolve-Path -LiteralPath $LogDir).Path

$native = @'
using System;
using System.Runtime.InteropServices;
public static class FipsLoadNative {
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr LoadLibraryW(string lpFileName);
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool FreeLibrary(IntPtr hModule);
}
'@
Add-Type -TypeDefinition $native

function Invoke-QuietInstall {
    param(
        [Parameter(Mandatory)][ValidateSet('install', 'remove')][string]$Action,
        [Parameter(Mandatory)][string]$LogName
    )
    $log = Join-Path $LogDir $LogName
    $flag = if ($Action -eq 'install') { '/i' } else { '/x' }
    $proc = Start-Process -FilePath "$env:SystemRoot\System32\msiexec.exe" -ArgumentList @(
        $flag, $MsiPath, '/qn', '/l*v', $log
    ) -Wait -PassThru
    if ($proc.ExitCode -eq 0 -or $proc.ExitCode -eq 3010) { return }
    if ($Action -eq 'remove' -and $proc.ExitCode -eq 1605) { return }
    throw "msiexec $Action exited $($proc.ExitCode). Log: $log"
}

function Get-Dumpbin {
    $cmd = Get-Command dumpbin.exe -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path -LiteralPath $vswhere)) { throw 'dumpbin.exe and vswhere.exe were not found' }
    $root = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if (-not $root) { throw 'Visual Studio C++ tools were not found' }
    $dumpbin = Get-ChildItem -Path (Join-Path $root 'VC\Tools\MSVC') -Recurse -Filter dumpbin.exe |
        Where-Object { $_.FullName -match '\\Hostx64\\x64\\dumpbin.exe$' } |
        Select-Object -First 1
    if (-not $dumpbin) { throw 'dumpbin.exe was not found under the Visual Studio C++ tools' }
    return $dumpbin.FullName
}

try {
    Invoke-QuietInstall -Action install -LogName 'fips-load-install.log'
    $driverKey = 'HKLM:\SOFTWARE\ODBC\ODBCINST.INI\Snowflake ODBC'
    if (-not (Test-Path -LiteralPath $driverKey)) { throw "missing $driverKey" }
    $driver = (Get-ItemProperty -LiteralPath $driverKey).Driver
    if (-not (Test-Path -LiteralPath $driver)) { throw "installed driver is missing: $driver" }
    $bin = Split-Path -Parent $driver
    $crypto = @(Get-ChildItem -LiteralPath $bin -Filter 'aws_lc_fips_*_crypto.dll' -File)
    if ($crypto.Count -ne 1) {
        throw "expected one AWS-LC crypto DLL in $bin, found $($crypto.Count)"
    }

    $dumpbin = Get-Dumpbin
    $dependents = & $dumpbin /dependents $crypto[0].FullName
    if ($LASTEXITCODE -ne 0) { throw "dumpbin /dependents exited $LASTEXITCODE" }
    $imports = @($dependents | ForEach-Object { $_.Trim() } | Where-Object { $_ -match '^[A-Za-z0-9_.+-]+\.dll$' })
    $system32 = Join-Path $env:SystemRoot 'System32'
    foreach ($import in $imports) {
        $inBin = Test-Path -LiteralPath (Join-Path $bin $import)
        $inSystem = Test-Path -LiteralPath (Join-Path $system32 $import)
        $apiSet = $import -like 'api-ms-*.dll' -or $import -like 'ext-ms-*.dll'
        if (-not $inBin -and -not $inSystem -and -not $apiSet) {
            throw "crypto DLL imports $import, which is neither in $bin nor in $system32"
        }
    }

    $scrubbed = @(
        $env:PATH -split ';' | Where-Object {
            $_ -and -not [string]::Equals($_.TrimEnd('\'), $bin, [StringComparison]::OrdinalIgnoreCase)
        }
    )
    $env:PATH = $scrubbed -join ';'
    Push-Location $env:TEMP
    try {
        $handle = [FipsLoadNative]::LoadLibraryW($driver)
        if ($handle -eq [IntPtr]::Zero) {
            $errorCode = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
            throw "LoadLibrary($driver) failed with Win32 $errorCode"
        }
        [void][FipsLoadNative]::FreeLibrary($handle)
    }
    finally {
        Pop-Location
    }
}
finally {
    Invoke-QuietInstall -Action remove -LogName 'fips-load-remove.log'
}

Write-Host "FIPS driver loaded from $driver with its directory off PATH"
