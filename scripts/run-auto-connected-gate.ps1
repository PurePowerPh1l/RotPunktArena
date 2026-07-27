# Connected-Gate Autoconnect Lab — Soft-Toast vs. idle Gerät.
#
#   powershell -ExecutionPolicy Bypass -File scripts\run-auto-connected-gate.ps1
#   powershell -ExecutionPolicy Bypass -File scripts\run-auto-connected-gate.ps1 -Force
#
# Ohne -Force: connected=false → SKIP (kein Connect). Mit -Force: trotzdem A (Toast?).

param(
    [switch]$Force
)

$ErrorActionPreference = "Stop"
$tauri = Join-Path $PSScriptRoot "..\apps\desktop\src-tauri" | Resolve-Path

Write-Host "bt_auto_connected_gate force=$Force" -ForegroundColor Cyan
Write-Host "Gerät idle lassen → ohne -Force: SKIP. Soft-Toast notieren." -ForegroundColor DarkYellow
Write-Host "Warm/connected → Connect. Soft-Toast notieren." -ForegroundColor DarkYellow

Push-Location $tauri
try {
    if ($Force) {
        cargo run --bin bt_auto_connected_gate --features rfcomm -- force
    } else {
        cargo run --bin bt_auto_connected_gate --features rfcomm
    }
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
