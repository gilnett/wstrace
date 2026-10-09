<#
.SYNOPSIS
    Builds and packages wstrace release zip archives locally in a clean dist/ folder.
.DESCRIPTION
    1. Reads version from Cargo.toml.
    2. Builds release binary (cargo build --release).
    3. Packages wstrace.exe, README.md, and LICENSE into dist/wstrace-v<version>-windows-x86_64.zip.
    4. Computes SHA-256 hash and outputs it for Scoop/Winget manifest updating.
#>

[CmdletBinding()]
param(
    [string]$Target = "x86_64-pc-windows-msvc"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

# 1. Read Version
$cargoToml = Get-Content (Join-Path $Root "Cargo.toml") -Raw
if ($cargoToml -match '(?m)^version\s*=\s*"([^"]+)"') {
    $version = $matches[1]
} else {
    throw "Could not determine version from Cargo.toml"
}

Write-Host "==> Packaging wstrace v$version for target $Target..." -ForegroundColor Cyan

# 2. Compile Release
Write-Host "--> Compiling release binary..." -ForegroundColor White
cargo build --release

$binPath = Join-Path $Root "target\release\wstrace.exe"
if (-not (Test-Path $binPath)) {
    throw "Binary not found at $binPath"
}

# 3. Create dist directory
$distDir = Join-Path $Root "dist"
if (Test-Path $distDir) {
    Remove-Item -Path $distDir -Recurse -Force
}
New-Item -ItemType Directory -Path $distDir -Force | Out-Null

$zipName = "wstrace-v$version-windows-x86_64.zip"
$zipPath = Join-Path $distDir $zipName
$stagingDir = Join-Path $distDir "staging"
New-Item -ItemType Directory -Path $stagingDir -Force | Out-Null

Copy-Item $binPath (Join-Path $stagingDir "wstrace.exe")
Copy-Item (Join-Path $Root "README.md") (Join-Path $stagingDir "README.md")
Copy-Item (Join-Path $Root "LICENSE") (Join-Path $stagingDir "LICENSE")

Compress-Archive -Path "$stagingDir\*" -DestinationPath $zipPath -Force
Remove-Item -Path $stagingDir -Recurse -Force

# 4. Compute Hash
$hash = (Get-FileHash -Path $zipPath -Algorithm SHA256).Hash.ToLower()

Write-Host "`n========================================================" -ForegroundColor Green
Write-Host " RELEASE ARTIFACT CREATED" -ForegroundColor Green
Write-Host "========================================================" -ForegroundColor Green
Write-Host " Archive : dist\$zipName" -ForegroundColor White
Write-Host " SHA-256 : $hash" -ForegroundColor Yellow
Write-Host "========================================================" -ForegroundColor Green
