# Diagnose-only lab — NOT the product Owner/startup path.
# Compares A vs Nuclear Light vs Full Nuclear without changing ConnectionManager.
#
#   powershell -ExecutionPolicy Bypass -File scripts\run-start-pair-variants.ps1
#   powershell -ExecutionPolicy Bypass -File scripts\run-start-pair-variants.ps1 -Variant L1
#
# Soft-Toast: confirm with j/N. JSONL: logs/start_pair_variants.jsonl

param(
    [ValidateSet("A", "L1", "L2", "N", "all")]
    [string]$Variant = "all"
)

$ErrorActionPreference = "Stop"
$tauri = Join-Path $PSScriptRoot "..\apps\desktop\src-tauri" | Resolve-Path

Write-Host "bt_start_pair_variants (DIAGNOSE-ONLY) variant=$Variant" -ForegroundColor Cyan
Write-Host "Idle device: L1 often alreadyAuthenticated = Light useless for product start." -ForegroundColor DarkYellow

Push-Location $tauri
try {
    cargo run --bin bt_start_pair_variants --features rfcomm -- $Variant
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
