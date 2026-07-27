# A/B Autoconnect Lab — Soft-Toast Hypothese (kein PIN-Hook).
#
#   powershell -ExecutionPolicy Bypass -File scripts\run-auto-ab.ps1
#   powershell -ExecutionPolicy Bypass -File scripts\run-auto-ab.ps1 -Mode a
#   powershell -ExecutionPolicy Bypass -File scripts\run-auto-ab.ps1 -Mode b
#
# Watch Soft-Toast during each phase (Enter prompts in the bin).

param(
    [ValidateSet("a", "b", "both")]
    [string]$Mode = "both"
)

$ErrorActionPreference = "Stop"
$tauri = Join-Path $PSScriptRoot "..\apps\desktop\src-tauri" | Resolve-Path

Write-Host "bt_auto_ab mode=$Mode" -ForegroundColor Cyan
Write-Host "Ziel AN, Bond authenticated. Soft-Toast bei A vs B notieren." -ForegroundColor DarkYellow

Push-Location $tauri
try {
    cargo run --bin bt_auto_ab --features rfcomm -- $Mode
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
