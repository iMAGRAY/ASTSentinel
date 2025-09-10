<#
  Скрипт: scripts/net_check.ps1
  Назначение: Быстрый e2e‑тест сетевого доступа (DNS/TLS/HTTP/JSON).
  Выход: код 0 при успехе, ненулевой при сбое. Пишет краткий отчёт в stdout.
  Использует: curl.exe (Windows), доступ в интернет.
#>
param(
  [switch]$Verbose
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Test-Head($Url, [int]$ExpectedStatus = 200) {
  $result = curl.exe -I $Url 2>$null | Select-Object -First 1
  if (-not $result) { throw "HEAD $Url → пустой ответ" }
  if ($result -notmatch "\s$ExpectedStatus\s") {
    throw "HEAD $Url → неожиданный статус: $result"
  }
  if ($Verbose) { Write-Host "[OK] $Url → $result" }
}

function Test-Ipify() {
  $json = curl.exe -s https://api.ipify.org?format=json 2>$null
  if (-not $json) { throw 'ipify: пустой ответ' }
  $obj = $null
  try { $obj = $json | ConvertFrom-Json } catch { throw "ipify: не JSON: $json" }
  if (-not $obj.ip) { throw "ipify: нет поля ip: $json" }
  if ($Verbose) { Write-Host "[OK] ipify → $($obj.ip)" }
}

try {
  Test-Head 'https://example.com'
  Test-Head 'https://www.wikipedia.org'
  Test-Ipify
  Write-Output 'NETWORK CHECK: OK'
  exit 0
}
catch {
  Write-Error $_
  Write-Output 'NETWORK CHECK: FAIL'
  exit 2
}

