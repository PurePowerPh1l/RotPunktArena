# RFCOMM overall lab matrix: Nuclear + Bond-Gate Autoconnect.
# From repo root (Windows PowerShell 5.1 oder 7+):
#   powershell -ExecutionPolicy Bypass -File scripts/run-nuclear-matrix.ps1
#   powershell -File scripts/run-nuclear-matrix.ps1 -SkipHold
#   powershell -File scripts/run-nuclear-matrix.ps1 -Only cold,bondgate,timeout
#
# From apps/desktop/src-tauri:
#   powershell -File ..\..\..\scripts\run-nuclear-matrix.ps1
#
# Order (full run): cheap/negativ → bond-gate (Soft+Nuclear) → soft labs → nuclear stress.
# IDs: N1 cold | N6 timeout | N7 json | N10 bondgate | N8 auto | N9 autonuc | N2–N5 nuclear

param(
    [string]$Only = "",
    [switch]$SkipHold
)

$ErrorActionPreference = "Stop"
$ScriptDir = $PSScriptRoot
$Root = Split-Path -Parent $ScriptDir
if (-not (Test-Path (Join-Path $Root "apps\desktop\src-tauri\Cargo.toml"))) {
    $probe = Get-Location
    for ($i = 0; $i -lt 6; $i++) {
        $cand = Join-Path $probe.Path "apps\desktop\src-tauri\Cargo.toml"
        if (Test-Path $cand) {
            $Root = $probe.Path
            break
        }
        $cand2 = Join-Path $probe.Path "Cargo.toml"
        if ((Test-Path $cand2) -and ((Get-Content $cand2 -Raw) -match 'name = "reddot-desktop"')) {
            $Tauri = $probe.Path
            Set-Location $Tauri
            $Root = (Resolve-Path (Join-Path $Tauri "..\..\..")).Path
            break
        }
        $probe = Split-Path -Parent $probe.Path
        if (-not $probe) { break }
    }
}
$Tauri = Join-Path $Root "apps\desktop\src-tauri"
if (-not (Test-Path (Join-Path $Tauri "Cargo.toml"))) {
    throw "src-tauri nicht gefunden. Bitte aus Repo-Root: powershell -File scripts/run-nuclear-matrix.ps1"
}
Set-Location $Tauri
Write-Host "cwd=$Tauri"
Write-Host "RFCOMM matrix: Nuclear + Bond-Gate Autoconnect" -ForegroundColor Cyan

function Invoke-Lab([string]$Id, [string]$Bin, [string[]]$ExtraArgs = @()) {
    Write-Host ""
    Write-Host "======== $Id ($Bin) ========" -ForegroundColor Cyan
    $sw = [Diagnostics.Stopwatch]::StartNew()
    & cargo run --bin $Bin --features rfcomm @ExtraArgs
    $code = $LASTEXITCODE
    $sw.Stop()
    if ($code -ne 0) {
        Write-Host "FAIL $Id exit=$code elapsed=$($sw.Elapsed)" -ForegroundColor Red
        return $false
    }
    Write-Host "PASS $Id elapsed=$($sw.Elapsed)" -ForegroundColor Green
    return $true
}

function Settle([int]$Seconds, [string]$Why) {
    Write-Host ""
    Write-Host ("Settle {0}s - {1}..." -f $Seconds, $Why) -ForegroundColor DarkGray
    Start-Sleep -Seconds $Seconds
}

$want = @()
if ($Only) {
    $want = $Only.Split(",") | ForEach-Object { $_.Trim().ToLowerInvariant() }
}

$results = @()

function Should-Run([string]$id) {
    if ($want.Count -eq 0) { return $true }
    return $want -contains $id
}

# --- Tier A: cold + negativ (wenig Stack-Last) ---
if (Should-Run "cold") {
    $ok = Invoke-Lab "N1-cold-start" "bt_cold_start"
    $results += [pscustomobject]@{ Id = "N1"; Name = "Cold start no auto-link"; Pass = $ok }
}

if (Should-Run "timeout") {
    $ok = Invoke-Lab "N6-nuclear-timeout" "bt_nuclear_timeout"
    $results += [pscustomobject]@{ Id = "N6"; Name = "Nuclear unreachable -> clean error"; Pass = $ok }
}

if (Should-Run "json") {
    $ok = Invoke-Lab "N7-json-corrupt" "bt_json_corrupt"
    $results += [pscustomobject]@{ Id = "N7"; Name = "Corrupt known JSON no crash"; Pass = $ok }
}

# --- Tier B: Bond-Gate (Soft bei Bond, Nuclear bei Bond-weg) ---
if (Should-Run "bondgate") {
    if ($results.Count -gt 0) {
        Settle 5 "vor Bond-Gate (nach cold/negativ)"
    }
    $ok = Invoke-Lab "N10-bond-gate" "bt_bond_gate_matrix"
    $results += [pscustomobject]@{ Id = "N10"; Name = "Bond-Gate Soft/Nuclear staffelung"; Pass = $ok }
}

# --- Tier C: Soft-Labs (nach N10: Bond i.d.R. vorhanden) ---
if (Should-Run "auto") {
    Settle 5 "vor Soft-Autoconnect N8"
    $ok = Invoke-Lab "N8-auto-once" "bt_auto_once"
    $results += [pscustomobject]@{ Id = "N8"; Name = "Autoconnect once if bonded (else SKIP)"; Pass = $ok }
}

if (Should-Run "autonuc") {
    if (Should-Run "auto") {
        Settle 8 "zwischen N8 und N9 (nach Soft)"
    } elseif (Should-Run "bondgate") {
        Settle 5 "vor N9 (nach Bond-Gate)"
    }
    $ok = Invoke-Lab "N9-auto-then-nuclear" "bt_auto_then_nuclear"
    $results += [pscustomobject]@{ Id = "N9"; Name = "Soft if bond else nuclear fallback"; Pass = $ok }
}

# --- Tier D: Nuclear stress ---
if (Should-Run "reset") {
    if ((Should-Run "bondgate") -or (Should-Run "auto") -or (Should-Run "autonuc")) {
        Settle 5 "vor Nuclear-Core"
    }
    $ok = Invoke-Lab "N2-nuclear-core" "bt_reset_connect"
    $results += [pscustomobject]@{ Id = "N2"; Name = "Nuclear core Forget-Pair-RFCOMM"; Pass = $ok }
}

if (Should-Run "manager") {
    $ok = Invoke-Lab "N3-nuclear-manager" "bt_nuclear_smoke"
    $results += [pscustomobject]@{ Id = "N3"; Name = "Manager connect_known_nuclear + hold"; Pass = $ok }
}

if (Should-Run "twice") {
    $ok = Invoke-Lab "N4-nuclear-twice" "bt_nuclear_twice"
    $results += [pscustomobject]@{ Id = "N4"; Name = "Two Verbinden cycles"; Pass = $ok }
}

if (-not $SkipHold -and (Should-Run "product")) {
    $ok = Invoke-Lab "N5-product-smoke" "bt_product_smoke"
    $results += [pscustomobject]@{ Id = "N5"; Name = "Product smoke cold+nuclear+45s"; Pass = $ok }
}

Write-Host ""
Write-Host "======== MATRIX SUMMARY ========" -ForegroundColor Yellow
if ($results.Count -eq 0) {
    Write-Host "Keine Labs ausgewählt. -Only: cold,timeout,json,bondgate,auto,autonuc,reset,manager,twice,product" -ForegroundColor DarkYellow
    exit 2
}
$results | Format-Table -AutoSize
$failed = @($results | Where-Object { -not $_.Pass })
if ($failed.Count -gt 0) {
    Write-Host "$($failed.Count) failed" -ForegroundColor Red
    Write-Host "Hinweise:" -ForegroundColor DarkYellow
    Write-Host "  N8 SKIP (exit 0) ohne OS-Bond ist OK; FAIL nur wenn Bond da und Soft scheitert." -ForegroundColor DarkYellow
    Write-Host "  N10 muss Soft bei Bond + Gate->Nuclear bei Bond-weg belegen." -ForegroundColor DarkYellow
    Write-Host "  N9 muss Linked liefern (Soft oder Nuclear)." -ForegroundColor DarkYellow
    exit 1
}
Write-Host "All automated lab rows PASS" -ForegroundColor Green
exit 0
