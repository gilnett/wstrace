<#
.SYNOPSIS
    wstrace Local Quality, Security, and Packaging Audit Suite.
.DESCRIPTION
    Runs complete verification locally before committing or opening a PR:
    1. Code formatting check (rustfmt)
    2. Static analysis and linting (clippy -D warnings)
    3. Unit & performance tests (cargo test)
    4. Dependency security audit (cargo-audit / RustSec)
    5. Packaging manifests validation (Scoop JSON & Winget YAML)
    6. Version synchronization (Cargo.toml vs Scoop vs Winget)
    7. Anti-binary git hygiene check (no tracked or staged .exe / .zip / .dll / .pdb)
#>

[CmdletBinding()]
param(
    [switch]$SkipTests = $false
)

$ErrorActionPreference = "Continue"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

$PassCount = 0
$FailCount = 0

function Write-AuditHeader($title) {
    Write-Host "`n========================================================" -ForegroundColor Cyan
    Write-Host " [AUDIT] $title" -ForegroundColor Cyan
    Write-Host "========================================================" -ForegroundColor Cyan
}

function Report-Success($msg) {
    Write-Host " [PASS] $msg" -ForegroundColor Green
    $script:PassCount++
}

function Report-Failure($msg) {
    Write-Host " [FAIL] $msg" -ForegroundColor Red
    $script:FailCount++
}

function Report-Warn($msg) {
    Write-Host " [WARN] $msg" -ForegroundColor Yellow
}

Write-AuditHeader "Starting wstrace Quality & Security Audit"

# ----------------------------------------------------
# 1. Anti-Binary & Git Hygiene Check
# ----------------------------------------------------
Write-Host "`n--> [1/6] Inspecting Git tree for binary leaks..." -ForegroundColor White
$trackedBinaries = git ls-files | Where-Object { $_ -match '\.(exe|zip|msi|dll|pdb|tar|gz)$' }
if ($trackedBinaries) {
    Report-Failure "Tracked binary files detected in Git: $trackedBinaries"
} else {
    Report-Success "Git tree is clean: 0 binaries tracked."
}

# ----------------------------------------------------
# 2. Code Formatting (rustfmt)
# ----------------------------------------------------
Write-Host "`n--> [2/6] Verifying Rust formatting (cargo fmt --check)..." -ForegroundColor White
try {
    $fmtOut = & cargo fmt --check 2>&1
    if ($LASTEXITCODE -eq 0) {
        Report-Success "Code conforms to official Rust formatting."
    } else {
        Report-Failure "Formatting issues detected! Run 'cargo fmt' to fix.`n$fmtOut"
    }
} catch {
    Report-Failure "cargo fmt failed to execute: $_"
}

# ----------------------------------------------------
# 3. Static Analysis & Security Lints (clippy)
# ----------------------------------------------------
Write-Host "`n--> [3/6] Running Clippy linter with strict zero-warning policy..." -ForegroundColor White
try {
    $clippyOut = & cargo clippy --all-targets -- -D warnings 2>&1
    if ($LASTEXITCODE -eq 0) {
        Report-Success "Clippy passed: 0 warnings, 0 security/correctness anti-patterns."
    } else {
        Report-Failure "Clippy reported issues:`n$clippyOut"
    }
} catch {
    Report-Failure "cargo clippy failed to execute: $_"
}

# ----------------------------------------------------
# 4. Unit & Performance Tests
# ----------------------------------------------------
if (-not $SkipTests) {
    Write-Host "`n--> [4/6] Executing Unit & Performance tests..." -ForegroundColor White
    try {
        $testOut = & cargo test 2>&1
        if ($LASTEXITCODE -eq 0) {
            Report-Success "All tests passed with 0 failures."
        } else {
            Report-Failure "Tests failed:`n$testOut"
        }
    } catch {
        Report-Failure "cargo test failed to execute: $_"
    }
} else {
    Report-Warn "Skipping tests as requested by switch."
}

# ----------------------------------------------------
# 5. Dependency Security Audit (cargo-audit)
# ----------------------------------------------------
Write-Host "`n--> [5/6] Checking for dependency vulnerabilities (RustSec)..." -ForegroundColor White
$cargoAuditCmd = Get-Command "cargo-audit" -ErrorAction SilentlyContinue
if ($null -eq $cargoAuditCmd) {
    Report-Warn "cargo-audit is not installed locally. (It will still run automatically in CI)."
    Write-Host "       Tip: Run 'cargo install cargo-audit' to enable local CVE scanning." -ForegroundColor Gray
} else {
    try {
        $auditOut = & cargo audit 2>&1
        if ($LASTEXITCODE -eq 0) {
            Report-Success "Zero known vulnerabilities (RustSec database checked)."
        } else {
            Report-Failure "Vulnerability detected in dependencies:`n$auditOut"
        }
    } catch {
        Report-Failure "cargo audit failed: $_"
    }
}

# ----------------------------------------------------
# 6. Packaging Manifests & Version Sync Validation
# ----------------------------------------------------
Write-Host "`n--> [6/6] Validating Scoop & Winget manifests..." -ForegroundColor White

# Extract Cargo.toml version
$cargoContent = Get-Content (Join-Path $Root "Cargo.toml") -Raw
if ($cargoContent -match '(?m)^version\s*=\s*"([^"]+)"') {
    $cargoVersion = $matches[1]
    Write-Host "       Detected Cargo version: $cargoVersion" -ForegroundColor Gray
} else {
    $cargoVersion = $null
    Report-Failure "Could not extract version from Cargo.toml"
}

# Validate Scoop JSON
$scoopPath = Join-Path $Root "packaging\scoop\wstrace.json"
if (Test-Path $scoopPath) {
    try {
        $scoopJson = Get-Content $scoopPath -Raw | ConvertFrom-Json
        if (-not $scoopJson.version) {
            Report-Failure "Scoop manifest missing 'version' field."
        } elseif ($cargoVersion -and ($scoopJson.version -ne $cargoVersion)) {
            Report-Failure "Scoop version ($($scoopJson.version)) does not match Cargo.toml ($cargoVersion)."
        } else {
            Report-Success "Scoop manifest valid (version: $($scoopJson.version))."
        }
    } catch {
        Report-Failure "Scoop manifest is not valid JSON: $_"
    }
} else {
    Report-Failure "Scoop manifest not found at $scoopPath"
}

# Validate Winget YAML
$wingetPath = Join-Path $Root "packaging\winget\wstrace.yaml"
if (Test-Path $wingetPath) {
    $wingetContent = Get-Content $wingetPath -Raw
    $wingetVerMatch = [regex]::Match($wingetContent, '(?m)^PackageVersion:\s*(\S+)')
    $wingetLocaleMatch = [regex]::Match($wingetContent, '(?m)^PackageLocale:\s*(\S+)')
    
    if (-not $wingetVerMatch.Success) {
        Report-Failure "Winget manifest missing 'PackageVersion'."
    } elseif ($cargoVersion -and ($wingetVerMatch.Groups[1].Value -ne $cargoVersion)) {
        Report-Failure "Winget PackageVersion ($($wingetVerMatch.Groups[1].Value)) does not match Cargo.toml ($cargoVersion)."
    } else {
        Report-Success "Winget manifest version synchronized ($($wingetVerMatch.Groups[1].Value))."
    }

    if (-not $wingetLocaleMatch.Success) {
        Report-Failure "Winget manifest missing mandatory 'PackageLocale'."
    } else {
        Report-Success "Winget mandatory PackageLocale present ($($wingetLocaleMatch.Groups[1].Value))."
    }

    # Run winget validate if winget CLI is present
    $wingetCmd = Get-Command "winget" -ErrorAction SilentlyContinue
    if ($wingetCmd) {
        $wingetVal = & winget validate --manifest $wingetPath 2>&1
        if ($LASTEXITCODE -eq 0) {
            Report-Success "winget validate passed with 0 errors."
        } else {
            Report-Failure "winget validate failed:`n$wingetVal"
        }
    }
} else {
    Report-Failure "Winget manifest not found at $wingetPath"
}

# ----------------------------------------------------
# Summary
# ----------------------------------------------------
Write-Host "`n========================================================" -ForegroundColor Cyan
Write-Host " AUDIT SUMMARY: $PassCount passed, $FailCount failed" -ForegroundColor $(if ($FailCount -eq 0) { "Green" } else { "Red" })
Write-Host "========================================================" -ForegroundColor Cyan

if ($FailCount -gt 0) {
    exit 1
} else {
    exit 0
}
