$ErrorActionPreference = 'Stop'

param(
  [string]$TargetDir = "$env:USERPROFILE\.claude\hooks\rust_validation_hooks\prompts"
)

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $here '..')
$src = Join-Path $repoRoot 'prompts'

Write-Host "Installing prompts from: $src" -ForegroundColor Cyan
Write-Host "To: $TargetDir" -ForegroundColor Cyan

New-Item -ItemType Directory -Force -Path $TargetDir | Out-Null
Copy-Item -Recurse -Force -Path (Join-Path $src '*') -Destination $TargetDir

Write-Host "Done." -ForegroundColor Green

