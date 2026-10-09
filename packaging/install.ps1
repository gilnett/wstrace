#Requires -Version 5.1
<#
.SYNOPSIS
    install.ps1 - Automated local installer for wstrace on Windows
.DESCRIPTION
    Installs wstrace.exe to the user binary directory ($HOME\.local\bin)
    and persists the directory in the User PATH environment variable.
#>
[CmdletBinding()]
param(
    [string]$SourceExe = "",
    [string]$InstallDir = "$HOME\.local\bin"
)

$ErrorActionPreference = "Stop"

Write-Host "[INFO] Starting wstrace installation..." -ForegroundColor Cyan

$candidates = @(
    $SourceExe,
    "$HOME\.cargo\bin\wstrace.exe",
    "$PSScriptRoot\..\target\release\wstrace.exe",
    "$PSScriptRoot\target\release\wstrace.exe",
    ".\target\release\wstrace.exe",
    "$PSScriptRoot\..\target\debug\wstrace.exe",
    ".\target\debug\wstrace.exe"
)

$found = $null
foreach ($c in $candidates) {
    if ($c -and (Test-Path $c)) {
        $found = (Resolve-Path $c).Path
        break
    }
}

if (-not $found) {
    Write-Host "[ERROR] Could not locate wstrace.exe. Run 'cargo build --release' first." -ForegroundColor Red
    exit 1
}
$SourceExe = $found

# 1. Create target installation directory if missing
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Write-Host "[OK] Created directory: $InstallDir" -ForegroundColor Green
}

# 2. Copy binary
$targetPath = Join-Path $InstallDir "wstrace.exe"
Copy-Item -Path $SourceExe -Destination $targetPath -Force
Write-Host "[OK] Binary installed to: $targetPath" -ForegroundColor Green

# 3. Persist User PATH environment variable
$pathsToAdd = @($InstallDir)
$cargoBin = "$HOME\.cargo\bin"
if (Test-Path $cargoBin) {
    $pathsToAdd += $cargoBin
}

$userPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
if (-not $userPath) { $userPath = "" }
$userPathSegments = $userPath -split ';'

foreach ($p in $pathsToAdd) {
    if ($userPathSegments -notcontains $p) {
        $userPath = if ($userPath) { "$userPath;$p" } else { $p }
        [Environment]::SetEnvironmentVariable("Path", $userPath, [EnvironmentVariableTarget]::User)
        $env:Path = "$env:Path;$p"
        Write-Host "[OK] Added $p to User PATH." -ForegroundColor Green
    } else {
        Write-Host "[OK] $p is already present in User PATH." -ForegroundColor Gray
    }
}

# 4. Verification test
$ver = & "$targetPath" --version
Write-Host "[OK] Verification complete: $ver is ready to use." -ForegroundColor Cyan
