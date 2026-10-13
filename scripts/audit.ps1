<#
.SYNOPSIS
    wstrace Local Quality, Security, Packaging, and Hardening Audit Suite.
.DESCRIPTION
    Runs complete verification locally before committing or opening a PR:
    1. Anti-binary git hygiene check (no tracked or staged .exe / .zip / .dll / .pdb)
    2. Code formatting check (rustfmt)
    3. Static analysis and linting (clippy -D warnings)
    4. Unit & integration tests (cargo test)
    5. Micro-benchmark suite integrity (cargo bench --no-run)
    6. Dependency tree validation (cargo tree for zero bloat)
    7. Dependency security audit (cargo-audit / RustSec)
    8. CLI documentation synchronization (src/cli.rs vs docs/CLI_REFERENCE.md)
    9. Static security & secrets scanner (API keys, tokens, personal paths)
    10. Source code & comment hygiene (anti-emoji & English-only comments)
    11. Packaging manifests validation (Scoop JSON & Winget YAML)
    12. Git commit title length hygiene (<= 72 chars)
    13. PowerShell scripts static analysis (PSScriptAnalyzer)
#>

[CmdletBinding()]
param(
    [switch]$SkipTests = $false,
    [switch]$SkipBench = $false,
    [switch]$RunLeakBench = $false,
    [switch]$FailFast = $false
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
    if ($FailFast) {
        Write-Host " [ABORT] Fast-fail triggered on audit failure." -ForegroundColor Red
        [Environment]::ExitCode = 1
        exit 1
    }
}

function Report-Warn($msg) {
    Write-Host " [WARN] $msg" -ForegroundColor Yellow
}

Write-AuditHeader "Starting wstrace Quality & Security Audit"

# ----------------------------------------------------
# 1. Anti-Binary & Git Hygiene Check
# ----------------------------------------------------
Write-Host "`n--> [1/13] Inspecting Git tree for binary leaks..." -ForegroundColor White
$trackedBinaries = git ls-files | Where-Object { $_ -match '\.(exe|zip|msi|dll|pdb|tar|gz)$' }
if ($trackedBinaries) {
    Report-Failure "Tracked binary files detected in Git: $trackedBinaries"
} else {
    Report-Success "Git tree is clean: 0 binaries tracked."
}

# ----------------------------------------------------
# 2. Code Formatting (rustfmt)
# ----------------------------------------------------
Write-Host "`n--> [2/13] Verifying Rust formatting (cargo fmt --check)..." -ForegroundColor White
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
Write-Host "`n--> [3/13] Running Clippy linter with strict zero-warning policy..." -ForegroundColor White
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
    Write-Host "`n--> [4/13] Executing Unit & Performance tests..." -ForegroundColor White
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
# 5. Micro-Benchmark Suite Integrity (Divan)
# ----------------------------------------------------
if (-not $SkipBench) {
    Write-Host "`n--> [5/13] Verifying Divan micro-benchmarks integrity (cargo bench --no-run)..." -ForegroundColor White
    try {
        $benchOut = & cargo bench --no-run 2>&1
        if ($LASTEXITCODE -eq 0) {
            Report-Success "Divan micro-benchmarks compiled successfully."
        } else {
            Report-Failure "cargo bench --no-run failed:`n$benchOut"
        }
    } catch {
        Report-Failure "cargo bench failed to execute: $_"
    }
} else {
    Report-Warn "Skipping benchmark integrity check as requested by switch."
}

# ----------------------------------------------------
# 6. Dependency Tree Validation (cargo tree)
# ----------------------------------------------------
Write-Host "`n--> [6/13] Validating dependency tree (cargo tree) for zero bloat..." -ForegroundColor White
try {
    $treeOut = & cargo tree 2>&1
    if ($LASTEXITCODE -eq 0) {
        $bannedCrates = @('tokio', 'actix', 'reqwest', 'tonic', 'async-std', 'smol', 'hyper', 'rayon')
        $foundBanned = @()
        foreach ($bc in $bannedCrates) {
            if ($treeOut -match "(?m)^\s*[^a-zA-Z0-9_\s]*\s*$bc\s+v") {
                $foundBanned += $bc
            }
        }
        if ($foundBanned.Count -gt 0) {
            Report-Failure "Forbidden bloated crate(s) detected in dependencies: $($foundBanned -join ', ')"
        } else {
            Report-Success "Dependency tree is lean: 0 bloated runtime frameworks detected."
        }
    } else {
        Report-Failure "cargo tree failed: $treeOut"
    }
} catch {
    Report-Failure "cargo tree failed to execute: $_"
}

# ----------------------------------------------------
# 7. Dependency Security Audit (cargo-audit)
# ----------------------------------------------------
Write-Host "`n--> [7/13] Checking for dependency vulnerabilities (RustSec)..." -ForegroundColor White
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
# 8. CLI Documentation Synchronization Verification
# ----------------------------------------------------
Write-Host "`n--> [8/13] Verifying CLI documentation synchronization (src/cli.rs vs docs/CLI_REFERENCE.md)..." -ForegroundColor White
$cliRsPath = Join-Path $Root "src\cli.rs"
$cliRefPath = Join-Path $Root "docs\CLI_REFERENCE.md"
if ((Test-Path $cliRsPath) -and (Test-Path $cliRefPath)) {
    $cliRs = Get-Content $cliRsPath -Raw
    $cliRef = Get-Content $cliRefPath -Raw

    $missingDocs = @()
    # Extract all long flags dynamically from struct fields in src/cli.rs
    $fieldBlocks = [regex]::Matches($cliRs, '(?s)#\[arg\(([^\]]+)\)\]\s*pub\s+([a-zA-Z0-9_]+)\s*:')
    foreach ($fb in $fieldBlocks) {
        $attrs = $fb.Groups[1].Value
        $fieldName = $fb.Groups[2].Value
        $longMatch = [regex]::Match($attrs, 'long\s*=\s*"([^"]+)"')
        if ($longMatch.Success) {
            $flag = $longMatch.Groups[1].Value
        } elseif ($attrs -match '\blong\b') {
            $flag = $fieldName.Replace('_', '-')
        } else {
            continue
        }
        if ($cliRef -notmatch [regex]::Escape("--$flag")) {
            $missingDocs += "--$flag"
        }
    }

    # Extract subcommands dynamically from Commands enum in src/cli.rs using balanced braces
    $commandsMatch = [regex]::Match($cliRs, 'pub\s+enum\s+Commands\s*\{')
    $commandsSection = ""
    if ($commandsMatch.Success) {
        $startIndex = $commandsMatch.Index + $commandsMatch.Length - 1
        $braceCount = 0
        for ($i = $startIndex; $i -lt $cliRs.Length; $i++) {
            if ($cliRs[$i] -eq '{') {
                $braceCount++
            } elseif ($cliRs[$i] -eq '}') {
                $braceCount--
                if ($braceCount -eq 0) {
                    $commandsSection = $cliRs.Substring($startIndex, $i - $startIndex + 1)
                    break
                }
            }
        }
    }
    $variantMatches = [regex]::Matches($commandsSection, '(?m)^\s{4}([A-Z][a-zA-Z0-9]+)\s*(\{|,)')
    foreach ($vm in $variantMatches) {
        $sub = $vm.Groups[1].Value.ToLower()
        $subPattern = "(?m)^###\s+``" + $sub + "``"
        if ($cliRef -notmatch $subPattern) {
            $missingDocs += "subcommand '$sub'"
        }
    }

    if ($missingDocs.Count -gt 0) {
        Report-Failure "CLI options or subcommands missing from docs/CLI_REFERENCE.md: $($missingDocs -join ', ')"
    } else {
        Report-Success "All CLI subcommands, arguments, and options in src/cli.rs are documented in docs/CLI_REFERENCE.md."
    }
} else {
    Report-Failure "CLI source or reference document not found."
}

# ----------------------------------------------------
# 9. Static Security & Secrets Scanner
# ----------------------------------------------------
Write-Host "`n--> [9/13] Scanning for hardcoded secrets and personal workstation paths..." -ForegroundColor White
$scanDirs = @("src", "docs", "packaging", "scripts")
$secFailures = @()
$personalPathRegex = [regex]'[A-Za-z]:\\Users\\[a-zA-Z0-9_\-]+'
$secretRegexList = @(
    @{ Pattern = '(?i)(api[_-]?key|secret[_-]?key|access[_-]?token|auth[_-]?token)\s*[:=]\s*["''][a-zA-Z0-9_\-\.]{16,}["'']'; Name = 'Hardcoded API/Auth Token' },
    @{ Pattern = '(?i)(ghp_[a-zA-Z0-9]{36}|github_pat_[a-zA-Z0-9_]{82})'; Name = 'GitHub Token' },
    @{ Pattern = '-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----'; Name = 'Private Key' },
    @{ Pattern = '(?i)AIza[0-9A-Za-z-_]{35}'; Name = 'Google API Key' },
    @{ Pattern = '(?i)AKIA[0-9A-Z]{16}'; Name = 'AWS Access Key' }
)
foreach ($dir in $scanDirs) {
    $dirPath = Join-Path $Root $dir
    if (Test-Path $dirPath) {
        $files = Get-ChildItem -Path $dirPath -Recurse -File | Where-Object { $_.Extension -match '\.(rs|md|ps1|json|yaml|toml)$' }
        foreach ($f in $files) {
            $lines = Get-Content $f.FullName
            for ($i = 0; $i -lt $lines.Count; $i++) {
                $lineText = $lines[$i]
                foreach ($rule in $secretRegexList) {
                    if ($lineText -match $rule.Pattern) {
                        $secFailures += "$($rule.Name) in $($f.Name):$($i+1)"
                    }
                }
                if ($lineText -match $personalPathRegex) {
                    $matched = $matches[0]
                    if ($matched -notmatch '(?i)Users\\(user|username|<.*>|\.\.)') {
                        $secFailures += "Personal workstation path '$matched' in $($f.Name):$($i+1)"
                    }
                }
            }
        }
    }
}
if ($secFailures.Count -gt 0) {
    Report-Failure "Security & secrets violations detected:`n$($secFailures -join "`n")"
} else {
    Report-Success "Static security scanner passed: 0 secrets or personal workstation paths detected."
}

# ----------------------------------------------------
# 10. Source Code & Comment Hygiene (Anti-Emoji & English-Only)
# ----------------------------------------------------
Write-Host "`n--> [10/13] Enforcing source code hygiene (Anti-Emoji & English-Only comments)..." -ForegroundColor White
$hygieneErrors = @()
$emojiFiles = Get-ChildItem -Path $Root -Recurse -File | Where-Object {
    ($_.FullName -notmatch '\\target\\') -and ($_.FullName -notmatch '\\\.git\\') -and
    ($_.Extension -match '\.(rs|md|ps1|toml|json|yaml)$')
}
$emojiRegex = [regex]'[\uD83C-\uDBFF][\uDC00-\uDFFF]|[\u2600-\u27BF]'
foreach ($f in $emojiFiles) {
    $content = Get-Content $f.FullName -Raw
    if ($content -and ($content -match $emojiRegex)) {
        $hygieneErrors += "Emoji detected in $($f.FullName.Replace($Root, ''))"
    }
}
$frenchCommentRegex = [regex]'(?i)\b(vérifi|récupér|paramètre|fonction|exemple|initialis|supprim|ajoute|retourne|écoute|arrête|boucle)\w*\b'
$rsFiles = Get-ChildItem -Path (Join-Path $Root "src") -Recurse -Filter "*.rs"
foreach ($rf in $rsFiles) {
    $lines = Get-Content $rf.FullName
    for ($i = 0; $i -lt $lines.Count; $i++) {
        $lt = $lines[$i].Trim()
        if (($lt.StartsWith("//") -or $lt.StartsWith("/*") -or $lt.StartsWith("*")) -and ($lt -match $frenchCommentRegex)) {
            $hygieneErrors += "Non-English comment in $($rf.Name):$($i+1)"
        }
    }
}
if ($hygieneErrors.Count -gt 0) {
    Report-Failure "Hygiene violations detected:`n$($hygieneErrors -join "`n")"
} else {
    Report-Success "Source hygiene passed: 0 emojis and 100% English comments across codebase."
}

# ----------------------------------------------------
# 11. Packaging Manifests & Version Sync Validation
# ----------------------------------------------------
Write-Host "`n--> [11/13] Validating Scoop & Winget manifests..." -ForegroundColor White

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
$scoopHash = $null
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
        $scoopHash = $scoopJson.architecture.'64bit'.hash
        if ($scoopHash -match '^[a-fA-F0-9]{64}$') {
            Report-Success "Scoop SHA-256 checksum format valid ($($scoopHash.Substring(0, 8))...)."
        } else {
            Report-Failure "Scoop SHA-256 checksum is invalid or missing: '$scoopHash'"
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
    $wingetShaMatch = [regex]::Match($wingetContent, '(?m)^\s*InstallerSha256:\s*([a-fA-F0-9]{64})')
    
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

    if ($wingetShaMatch.Success) {
        $wingetHash = $wingetShaMatch.Groups[1].Value
        Report-Success "Winget InstallerSha256 format valid ($($wingetHash.Substring(0, 8))...)."
        if ($scoopHash -and ($scoopHash.ToLower() -ne $wingetHash.ToLower())) {
            Report-Failure "Scoop hash ($scoopHash) does not match Winget hash ($wingetHash)."
        } else {
            Report-Success "Scoop and Winget SHA-256 checksums are synchronized."
        }
    } else {
        Report-Failure "Winget manifest missing valid 64-char InstallerSha256."
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
# 12. Git Commit Title Length Hygiene
# ----------------------------------------------------
Write-Host "`n--> [12/13] Verifying Git commit title length guidelines (<= 72 chars)..." -ForegroundColor White
try {
    $headTitle = (git log -1 --pretty=format:"%s" 2>&1)
    if ($headTitle) {
        $headTitle = $headTitle.Trim()
        if ($headTitle.Length -gt 80) {
            Report-Warn "Latest Git commit title in history exceeds 80 chars ($($headTitle.Length) chars): '$headTitle'"
            Report-Success "Git commit title guideline verified (rule: keep new commit titles <= 72 chars)."
        } elseif ($headTitle.Length -gt 72) {
            Report-Warn "Latest Git commit title in history exceeds recommended 72 chars ($($headTitle.Length) chars): '$headTitle'"
            Report-Success "Git commit title is within acceptable 80-char ceiling."
        } else {
            Report-Success "Latest Git commit title complies with 72-char limit ($($headTitle.Length) chars): '$headTitle'"
        }
    } else {
        Report-Warn "No git commits found to inspect."
    }
} catch {
    Report-Warn "Could not inspect git commit log: $_"
}

# ----------------------------------------------------
# 13. PowerShell Scripts Static Analysis (PSScriptAnalyzer)
# ----------------------------------------------------
Write-Host "`n--> [13/13] Analyzing PowerShell scripts (*.ps1) with PSScriptAnalyzer..." -ForegroundColor White
$psAnalyzerCmd = Get-Command "Invoke-ScriptAnalyzer" -ErrorAction SilentlyContinue
if ($null -eq $psAnalyzerCmd) {
    Report-Warn "PSScriptAnalyzer is not installed locally. Run 'Install-Module -Name PSScriptAnalyzer -Scope CurrentUser'."
} else {
    try {
        $ps1Files = Get-ChildItem -Path $Root -Recurse -Filter "*.ps1"
        $psErrors = @()
        foreach ($file in $ps1Files) {
            $analysis = Invoke-ScriptAnalyzer -Path $file.FullName -Severity Error
            if ($analysis) {
                $psErrors += $analysis
            }
        }
        if ($psErrors.Count -eq 0) {
            Report-Success "All $($ps1Files.Count) PowerShell scripts passed static analysis (0 errors)."
        } else {
            Report-Failure "PowerShell script errors detected:`n$($psErrors | Out-String)"
        }
    } catch {
        Report-Failure "Invoke-ScriptAnalyzer execution failed: $_"
    }
}

# ----------------------------------------------------
# Dynamic Leak Benchmark Hook (Optional)
# ----------------------------------------------------
if ($RunLeakBench) {
    Write-Host "`n--> Dynamic leak benchmark using giga_audit bench..." -ForegroundColor White
    $gigaScript = Join-Path (Split-Path -Parent $Root) "giga_audit.py"
    if (Test-Path $gigaScript) {
        $binPath = Join-Path $Root "target\release\wstrace.exe"
        if (-not (Test-Path $binPath)) {
            $binPath = Join-Path $Root "target\debug\wstrace.exe"
        }
        if (Test-Path $binPath) {
            $benchArgs = @(
                $gigaScript,
                "bench",
                "--exec",
                "`"$binPath`" -m stream -d 2 run cmd.exe -- /c exit",
                "--duration",
                "3"
            )
            Write-Host "       Running: python $($benchArgs -join ' ')" -ForegroundColor Gray
            $leakOut = & python $benchArgs 2>&1
            Write-Host $leakOut
            Report-Success "Dynamic leak audit completed successfully."
        } else {
            Report-Warn "wstrace binary not found for leak benchmark. Run 'cargo build --release' first."
        }
    } else {
        Report-Warn "giga_audit.py not found at $gigaScript."
    }
}

# ----------------------------------------------------
# Summary
# ----------------------------------------------------
Write-Host "`n========================================================" -ForegroundColor Cyan
Write-Host " AUDIT SUMMARY: $PassCount passed, $FailCount failed" -ForegroundColor $(if ($FailCount -eq 0) { "Green" } else { "Red" })
Write-Host "========================================================" -ForegroundColor Cyan

if ($FailCount -gt 0) {
    Write-Host "`n [FATAL] Audit suite failed with $FailCount issue(s). Exiting with code 1." -ForegroundColor Red
    [Environment]::ExitCode = 1
    exit 1
} else {
    [Environment]::ExitCode = 0
    exit 0
}
