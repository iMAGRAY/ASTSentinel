$ErrorActionPreference = 'Stop'

param(
  [string]$HooksRoot = "$env:USERPROFILE\.claude\hooks\rust_validation_hooks",
  [string]$OutputDir = "$env:USERPROFILE\.claude",
  [string]$PretoolProvider = "",
  [string]$PosttoolProvider = ""
)

function Parse-DotEnv {
  param([string]$Path)
  $map = @{}
  if (-not (Test-Path $Path)) { return $map }
  Get-Content -LiteralPath $Path | ForEach-Object {
    $line = $_.Trim()
    if ([string]::IsNullOrWhiteSpace($line)) { return }
    if ($line.StartsWith('#')) { return }
    $ix = $line.IndexOf('=')
    if ($ix -lt 1) { return }
    $k = $line.Substring(0,$ix).Trim()
    $v = $line.Substring($ix+1).Trim().Trim('"').Trim("'")
    $map[$k] = $v
  }
  return $map
}

$envPath = Join-Path $HooksRoot '.env'
Write-Host "Reading .env from: $envPath" -ForegroundColor Cyan
$envMap = Parse-DotEnv -Path $envPath

function Get-FirstPresentKey {
  param([hashtable]$Map, [string[]]$Keys)
  foreach ($k in $Keys) { if ($Map.ContainsKey($k) -and $Map[$k]) { return $Map[$k] } }
  return ""
}

$openaiKey     = Get-FirstPresentKey $envMap @('OPENAI_API_KEY')
$anthropicKey  = Get-FirstPresentKey $envMap @('ANTHROPIC_API_KEY')
$googleKey     = Get-FirstPresentKey $envMap @('GOOGLE_API_KEY')
$xaiKey        = Get-FirstPresentKey $envMap @('XAI_API_KEY')

function Choose-Provider {
  param([string]$Prefer, [string]$OpenAI, [string]$Anthropic, [string]$Google, [string]$XAI)
  if ($Prefer) { return $Prefer }
  if ($OpenAI) { return 'openai' }
  if ($Anthropic) { return 'anthropic' }
  if ($Google) { return 'google' }
  if ($XAI) { return 'xai' }
  return 'xai'
}

$preProv  = Choose-Provider $PretoolProvider  $openaiKey $anthropicKey $googleKey $xaiKey
$postProv = Choose-Provider $PosttoolProvider $openaiKey $anthropicKey $googleKey $xaiKey

function Get-OrDefault {
  param([hashtable]$Map,[string]$Key,[string]$Default)
  if ($Map.ContainsKey($Key) -and $Map[$Key]) { return $Map[$Key] }
  return $Default
}

$preModel  = Get-OrDefault $envMap 'PRETOOL_MODEL'  ($preProv -eq 'openai' ? 'gpt-4.1-mini' : 'grok-code-fast-1')
$postModel = Get-OrDefault $envMap 'POSTTOOL_MODEL' ($postProv -eq 'openai' ? 'gpt-4.1-mini' : 'grok-code-fast-1')

$cfg = [ordered]@{
  pretool_provider       = $preProv
  posttool_provider      = $postProv
  pretool_model          = $preModel
  posttool_model         = $postModel
  openai_api_key         = $openaiKey
  anthropic_api_key      = $anthropicKey
  google_api_key         = $googleKey
  xai_api_key            = $xaiKey
  request_timeout_secs   = [int](Get-OrDefault $envMap 'REQUEST_TIMEOUT_SECS' '60')
  connect_timeout_secs   = [int](Get-OrDefault $envMap 'CONNECT_TIMEOUT_SECS' '30')
  max_tokens             = [int](Get-OrDefault $envMap 'MAX_TOKENS' '4000')
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$outPath = Join-Path $OutputDir '.hooks-config.json'
$cfg | ConvertTo-Json -Depth 4 | Out-File -LiteralPath $outPath -Encoding UTF8

Write-Host "Config written:" $outPath -ForegroundColor Green
Write-Host "Summary:" -ForegroundColor Yellow
Write-Host (" pretool_provider = {0}" -f $preProv)
Write-Host (" posttool_provider = {0}" -f $postProv)
Write-Host (" openai key  = {0}" -f ($(if($openaiKey){'present'}else{'missing'})))
Write-Host (" anthropic   = {0}" -f ($(if($anthropicKey){'present'}else{'missing'})))
Write-Host (" google      = {0}" -f ($(if($googleKey){'present'}else{'missing'})))
Write-Host (" xai         = {0}" -f ($(if($xaiKey){'present'}else{'missing'})))

Write-Host "To force hooks to use this config:" -ForegroundColor Cyan
Write-Host (" setx HOOKS_CONFIG_FILE \"{0}\"" -f $outPath)
Write-Host "(restart console to apply setx)"

