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
    "$PSScriptRoot\..\target\release\wstrace.exe",
    "$PSScriptRoot\target\release\wstrace.exe",
    ".\target\release\wstrace.exe",
    "$HOME\.cargo\bin\wstrace.exe",
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

$tempCleanupDir = $null

if (-not $found) {
    Write-Host "[INFO] Local binary not found. Fetching latest release from GitHub..." -ForegroundColor Cyan
    $repo = "gilnett/wstrace"
    $releasesApiUrl = "https://api.github.com/repos/$repo/releases/latest"
    $tempCleanupDir = Join-Path ([System.IO.Path]::GetTempPath()) "wstrace_install_$([System.Guid]::NewGuid().ToString('N'))"
    New-Item -ItemType Directory -Path $tempCleanupDir -Force | Out-Null
    $zipPath = Join-Path $tempCleanupDir "wstrace.zip"

    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Write-Host "       Querying GitHub releases for $repo..." -ForegroundColor Gray
        $release = Invoke-RestMethod -Uri $releasesApiUrl -Headers @{ "User-Agent" = "wstrace-installer" } -ErrorAction Stop
        
        $asset = $release.assets | Where-Object { $_.name -like "*windows-x86_64.zip" } | Select-Object -First 1
        if (-not $asset) {
            $downloadUrl = "https://github.com/$repo/releases/latest/download/wstrace-$($release.tag_name)-windows-x86_64.zip"
        } else {
            $downloadUrl = $asset.browser_download_url
        }

        Write-Host "       Downloading release artifact: $downloadUrl..." -ForegroundColor Gray
        Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath -ErrorAction Stop
        
        $extractDir = Join-Path $tempCleanupDir "extracted"
        Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force
        
        $extractedExe = Get-ChildItem -Path $extractDir -Recurse -Filter "wstrace.exe" | Select-Object -First 1
        if (-not $extractedExe) {
            throw "wstrace.exe not found inside the downloaded archive."
        }
        $found = $extractedExe.FullName
    } catch {
        Write-Host "[ERROR] Failed to download wstrace from GitHub releases: $_" -ForegroundColor Red
        Write-Host "        You can compile from source: 'cargo build --release'" -ForegroundColor Yellow
        if ($tempCleanupDir -and (Test-Path $tempCleanupDir)) {
            Remove-Item -Path $tempCleanupDir -Recurse -Force -ErrorAction SilentlyContinue
        }
        exit 1
    }
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

# Clean up temp files if created
if ($tempCleanupDir -and (Test-Path $tempCleanupDir)) {
    Remove-Item -Path $tempCleanupDir -Recurse -Force -ErrorAction SilentlyContinue
}

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
