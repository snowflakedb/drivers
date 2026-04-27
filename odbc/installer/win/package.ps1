<#
.SYNOPSIS
    Builds a Snowflake ODBC RS Driver MSI installer using the WiX Toolset v3.

.DESCRIPTION
    Invokes candle.exe (compiler) and light.exe (linker) from the WiX Toolset
    to produce an MSI installer for the Snowflake ODBC RS Driver.

.PARAMETER DriverBinDir
    Directory containing the built sfodbc.dll (e.g. target\release).

.PARAMETER Arch
    Target architecture: x64 or x86. Selects the matching WiX source file.
    Defaults to x64.

.PARAMETER BuildConfig
    Build configuration: release or debug. Used in the output filename.
    Defaults to release.

.PARAMETER VCRedistDir
    Directory containing the VC++ redistributable (vc_redist.x64.exe / vc_redist.x86.exe).
    Auto-detected from the Visual Studio installation if not specified.

.PARAMETER Version
    Version string for the product (e.g. 0.0.1-abc1234).
    Defaults to BASE_VERSION from odbc/version.sh with the git short hash appended.

.PARAMETER OutputDir
    Directory where the resulting MSI will be placed. Created if it doesn't exist.
    Defaults to build\.

.EXAMPLE
    .\odbc\installer\win\package.ps1 -DriverBinDir target\release -Arch x64

.EXAMPLE
    .\odbc\installer\win\package.ps1 -DriverBinDir target\i686-pc-windows-msvc\debug -Arch x86 -BuildConfig debug
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$DriverBinDir,

    [ValidateSet("x64", "x86")]
    [string]$Arch = "x64",

    [ValidateSet("release", "debug")]
    [string]$BuildConfig = "release",

    [string]$VCRedistDir,

    [string]$Version,

    [string]$OutputDir = "build"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$SourceDir = $PSScriptRoot | Split-Path | Split-Path | Split-Path
$WxsFile = Join-Path $PSScriptRoot "snowflake_odbc_${Arch}.wxs"

if (-not (Test-Path $WxsFile)) {
    throw "WiX source not found: $WxsFile"
}

# --- Version ---
if (-not $Version) {
    $versionLine = Get-Content (Join-Path $SourceDir "odbc\version.sh") -Raw
    if ($versionLine -match 'BASE_VERSION=(\S+)') {
        $baseVersion = $Matches[1]
    } else {
        throw "Could not parse BASE_VERSION from odbc/version.sh"
    }
    $commitHash = (git -C $SourceDir rev-parse --short HEAD 2>$null)
    if (-not $commitHash) { $commitHash = "unknown" }
    $Version = "${baseVersion}-${commitHash}"
}
$versionParts = ($Version -replace '-.*', '').Split('.')
while ($versionParts.Count -lt 3) { $versionParts += "0" }
$WixVersion = ($versionParts[0..2]) -join '.'

# --- Driver DLL ---
$DriverBinDir = (Resolve-Path $DriverBinDir).Path
if (-not (Test-Path (Join-Path $DriverBinDir "sfodbc.dll"))) {
    throw "sfodbc.dll not found in $DriverBinDir. Build the driver first."
}

# --- VC++ Redistributable ---
if (-not $VCRedistDir) {
    # Try VCINSTALLDIR env var first (set by vcvarsall.bat)
    if ($env:VCINSTALLDIR) {
        $VCRedistDir = Join-Path $env:VCINSTALLDIR "Redist\MSVC\v143"
    } else {
        # Auto-detect via vswhere
        $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
        if (Test-Path $vswhere) {
            $vsPath = & $vswhere -latest -property installationPath
            if ($vsPath) {
                $VCRedistDir = Join-Path $vsPath "VC\Redist\MSVC\v143"
            }
        }
    }
    if (-not $VCRedistDir -or -not (Test-Path $VCRedistDir)) {
        throw "VC++ Redistributable directory not found. Install Visual Studio or pass -VCRedistDir."
    }
}
$vcRedistExe = if ($Arch -eq "x64") { "vc_redist.x64.exe" } else { "vc_redist.x86.exe" }
if (-not (Test-Path (Join-Path $VCRedistDir $vcRedistExe))) {
    throw "$vcRedistExe not found in $VCRedistDir"
}

# --- Output ---
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$OutputDir = (Resolve-Path $OutputDir).Path
$configSuffix = if ($BuildConfig -eq "debug") { "-debug" } else { "" }

Write-Host "=== Building Snowflake ODBC RS Driver MSI ==="
Write-Host "  Architecture : $Arch"
Write-Host "  Config       : $BuildConfig"
Write-Host "  Version      : $Version (MSI ProductVersion: $WixVersion)"
Write-Host "  Driver dir   : $DriverBinDir"
Write-Host "  VCRedist dir : $VCRedistDir"
Write-Host "  Source dir   : $SourceDir"
Write-Host "  Output dir   : $OutputDir"

$ObjDir = Join-Path $OutputDir "wixobj"
New-Item -ItemType Directory -Force -Path $ObjDir | Out-Null

$WixObj = Join-Path $ObjDir "snowflake_odbc_${Arch}${configSuffix}.wixobj"
$MsiFile = Join-Path $OutputDir "snowflake_odbc_rs-${Version}${configSuffix}-${Arch}.msi"

$candleArch = if ($Arch -eq "x64") { "x64" } else { "x86" }

Write-Host "`n--- Compiling WiX source ---"
& candle.exe `
    -nologo `
    -arch $candleArch `
    -dProductVersion="$WixVersion" `
    -dFullVersion="$Version" `
    -dDriverBinDir="$DriverBinDir" `
    -dVCRedistDir="$VCRedistDir" `
    -dSourceDir="$SourceDir" `
    -out "$WixObj" `
    "$WxsFile"
if ($LASTEXITCODE -ne 0) { throw "candle.exe failed with exit code $LASTEXITCODE" }

Write-Host "`n--- Linking MSI ---"
& light.exe `
    -nologo `
    -ext WixUIExtension `
    -out "$MsiFile" `
    "$WixObj"
if ($LASTEXITCODE -ne 0) { throw "light.exe failed with exit code $LASTEXITCODE" }

Write-Host "`n=== Successfully created MSI: $MsiFile ==="
