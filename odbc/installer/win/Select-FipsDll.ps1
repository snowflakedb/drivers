function Get-BuildOutputFreshness {
    param(
        [Parameter(Mandatory)]
        [System.IO.FileInfo]$File,

        [Parameter(Mandatory)]
        [string]$Root
    )

    $rootPath = [System.IO.Path]::GetFullPath($Root).TrimEnd('\')
    $dir = $File.Directory
    while ($null -ne $dir) {
        $full = $dir.FullName
        $stamp = Join-Path $full 'invoked.timestamp'
        if (Test-Path -LiteralPath $stamp) {
            return (Get-Item -LiteralPath $stamp).LastWriteTimeUtc.Ticks
        }
        if ([string]::Equals($full.TrimEnd('\'), $rootPath, [StringComparison]::OrdinalIgnoreCase)) {
            break
        }
        $dir = $dir.Parent
    }
    return $File.LastWriteTimeUtc.Ticks
}

function Select-FipsDll {
    param(
        [Parameter(Mandatory)]
        [string]$DriverBinDir,

        [Parameter(Mandatory)]
        [string]$Filter,

        [switch]$Optional
    )

    $found = @(Get-ChildItem -Path $DriverBinDir -Recurse -File -Filter 'aws_lc_fips_*.dll' |
        Where-Object { $_.Name -like $Filter })
    if ($found.Count -eq 0) {
        if ($Optional) { return }
        throw "$Filter not found under $DriverBinDir"
    }
    $names = @($found | Select-Object -ExpandProperty Name -Unique)
    if ($names.Count -ne 1) {
        throw "Expected one $Filter name under $DriverBinDir, found: $($names -join ', ')"
    }
    if ($found.Count -eq 1) {
        return $found[0].FullName
    }

    $ranked = @($found | Sort-Object @{
        Expression = { Get-BuildOutputFreshness -File $_ -Root $DriverBinDir }
        Descending = $true
    })
    $newest = Get-BuildOutputFreshness -File $ranked[0] -Root $DriverBinDir
    $second = Get-BuildOutputFreshness -File $ranked[1] -Root $DriverBinDir
    if ($newest -eq $second) {
        $paths = ($ranked | ForEach-Object { $_.FullName }) -join ', '
        throw "more than one $($ranked[0].Name) under $DriverBinDir shares the newest invoked.timestamp: $paths"
    }
    return $ranked[0].FullName
}
