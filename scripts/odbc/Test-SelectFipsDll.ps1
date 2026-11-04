<#
.SYNOPSIS
    Checks that Select-FipsDll keeps the newest build-script output.
#>
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

. (Join-Path $PSScriptRoot '..\..\odbc\installer\win\Select-FipsDll.ps1')

$root = Join-Path ([System.IO.Path]::GetTempPath()) ("fips-dll-select-" + [guid]::NewGuid().ToString('n'))
New-Item -ItemType Directory -Force -Path $root | Out-Null

function New-CryptoCopy {
    param(
        [Parameter(Mandatory)][string]$Label,
        [Parameter(Mandatory)][datetime]$Stamp
    )
    $dir = Join-Path $root "build\aws-lc-fips-sys-$Label"
    $out = Join-Path $dir 'out'
    New-Item -ItemType Directory -Force -Path $out | Out-Null
    $dll = Join-Path $out 'aws_lc_fips_0_13_17_crypto.dll'
    [System.IO.File]::WriteAllBytes($dll, [byte[]](1, 2, 3))
    $stampPath = Join-Path $dir 'invoked.timestamp'
    [System.IO.File]::WriteAllText($stampPath, 'invoked')
    (Get-Item -LiteralPath $stampPath).LastWriteTimeUtc = $Stamp
    return $dll
}

try {
    $missing = Join-Path $root 'empty'
    New-Item -ItemType Directory -Force -Path $missing | Out-Null
    if ($null -ne (Select-FipsDll -DriverBinDir $missing -Filter 'aws_lc_fips_*_rust_wrapper.dll' -Optional)) {
        throw 'optional wrapper should be absent when the build emits none'
    }
    $threw = $false
    try {
        Select-FipsDll -DriverBinDir $missing -Filter 'aws_lc_fips_*_crypto.dll' | Out-Null
    }
    catch {
        $threw = $true
    }
    if (-not $threw) { throw 'a missing crypto DLL should fail the package' }

    $old = New-CryptoCopy -Label 'old' -Stamp ([datetime]::new(2020, 1, 1, 0, 0, 0, [DateTimeKind]::Utc))
    $new = New-CryptoCopy -Label 'new' -Stamp ([datetime]::new(2024, 6, 1, 0, 0, 0, [DateTimeKind]::Utc))
    $selected = Select-FipsDll -DriverBinDir $root -Filter 'aws_lc_fips_*_crypto.dll'
    if ($selected -ne $new) {
        throw "expected the newer crypto DLL`n selected: $selected`n newer: $new`n older: $old"
    }

    (Get-Item -LiteralPath (Join-Path $root 'build\aws-lc-fips-sys-old\invoked.timestamp')).LastWriteTimeUtc =
        [datetime]::new(2024, 6, 1, 0, 0, 0, [DateTimeKind]::Utc)
    $tied = $false
    try {
        Select-FipsDll -DriverBinDir $root -Filter 'aws_lc_fips_*_crypto.dll' | Out-Null
    }
    catch {
        $tied = $true
    }
    if (-not $tied) { throw 'two crypto DLLs with the same invoked.timestamp should fail' }
}
finally {
    Remove-Item -LiteralPath $root -Recurse -Force
}

Write-Host 'Select-FipsDll checks passed'
