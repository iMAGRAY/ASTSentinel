<#
  Простой e2e‑тест для scripts/net_check.ps1
  Критерий успеха: код возврата 0 и строка 'NETWORK CHECK: OK' в выводе.
#>
$ErrorActionPreference = 'Stop'

$cmd = Join-Path $PSScriptRoot '..' '..' 'scripts' 'net_check.ps1'
$out = & pwsh -NoLogo -NoProfile -File $cmd 2>&1
$code = $LASTEXITCODE

if ($code -ne 0) {
  Write-Error "net_check.ps1 вернул код $code. Вывод:`n$out"
  exit 1
}

if ($out -notmatch 'NETWORK CHECK: OK') {
  Write-Error "Ожидалась строка 'NETWORK CHECK: OK'. Фактический вывод:`n$out"
  exit 1
}

Write-Host 'test_net_check.ps1: OK'
exit 0

