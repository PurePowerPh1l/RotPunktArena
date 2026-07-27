# Thin wrapper → overall matrix, Bond-Gate only (N10).
#   powershell -ExecutionPolicy Bypass -File scripts\run-bond-gate-matrix.ps1
# Prefer full matrix:
#   powershell -ExecutionPolicy Bypass -File scripts\run-nuclear-matrix.ps1

& "$PSScriptRoot\run-nuclear-matrix.ps1" -Only bondgate
exit $LASTEXITCODE
