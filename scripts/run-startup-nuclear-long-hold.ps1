# Startup-Nuclear Langlauf 4h — außerhalb des Agents (überlebt Cursor-Session).
#
#   powershell -ExecutionPolicy Bypass -File scripts\run-startup-nuclear-long-hold.ps1
#   powershell -ExecutionPolicy Bypass -File scripts\run-startup-nuclear-long-hold.ps1 -Detach
#
# Log: logs/long-hold-4h.log
# Abbruch: Task-Manager → bt_startup_race.exe

param(
  [switch]$Detach,
  [int]$HoldSecs = 14400
)

$ErrorActionPreference = "Stop"
$root = "D:\Disag Reddot 2"
if ($PSScriptRoot) {
  $candidate = Resolve-Path (Join-Path $PSScriptRoot "..") -ErrorAction SilentlyContinue
  if ($candidate -and (Test-Path (Join-Path $candidate "apps\desktop\src-tauri"))) {
    $root = $candidate.Path
  }
}
$tauri = Join-Path $root "apps\desktop\src-tauri"
$logDir = Join-Path $root "logs"
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
$outLog = Join-Path $logDir "long-hold-4h.log"
$errLog = Join-Path $logDir "long-hold-4h.err.log"
$pidFile = Join-Path $logDir "long-hold-4h.pid.txt"

Set-Location $tauri
Write-Host "Building bt_startup_race…"
cargo build --bin bt_startup_race --features rfcomm | Out-Host

$candidates = @(
  (Join-Path $tauri "target\debug\bt_startup_race.exe"),
  (Join-Path $root "target\debug\bt_startup_race.exe")
) + @(
  Get-ChildItem -Path "$env:TEMP\cursor-sandbox-cache" -Recurse -Filter "bt_startup_race.exe" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending |
    Select-Object -ExpandProperty FullName
)
$exe = $candidates | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
if (-not $exe) { throw "bt_startup_race.exe not found" }

$started = Get-Date
$expectedEnd = $started.AddSeconds($HoldSecs)

"=== detach start $($started.ToString('o')) expected_end=$($expectedEnd.ToString('o')) hold_secs=$HoldSecs exe=$exe ===" |
  Set-Content $outLog
"" | Set-Content $errLog

Write-Host "RedDot AN lassen."
Write-Host "Log: $outLog"
Write-Host "Start: $($started.ToString('o'))"
Write-Host "Expected end: $($expectedEnd.ToString('o'))"

if ($Detach) {
  $p = Start-Process -FilePath "cmd.exe" `
    -ArgumentList "/c", "set REDOT_LONG_HOLD_SECS=$HoldSecs&& `"$exe`" long_hold 1>>`"$outLog`" 2>>`"$errLog`"" `
    -WorkingDirectory $tauri `
    -WindowStyle Hidden `
    -PassThru
  "cmd_pid=$($p.Id)`nstarted=$($started.ToString('o'))`nexpected_end=$($expectedEnd.ToString('o'))" |
    Set-Content $pidFile

  Start-Sleep -Seconds 10

  Write-Host "`n--- bt_startup_race nach 10s ---"
  Get-Process -Name bt_startup_race -ErrorAction SilentlyContinue |
    Select-Object Id, StartTime, Path |
    Format-Table -AutoSize |
    Out-Host

  Write-Host "Detach OK. Agent/Cursor können zu. Fortschritt: Get-Content '$outLog' -Tail 20 -Wait"
  exit 0
}

$env:REDOT_LONG_HOLD_SECS = "$HoldSecs"
& $exe long_hold 2>> $errLog | Tee-Object -FilePath $outLog -Append
Write-Host "EXIT $LASTEXITCODE $(Get-Date -Format o)"
