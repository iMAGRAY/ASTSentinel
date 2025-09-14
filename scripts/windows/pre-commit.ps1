param(
    [switch]$NoTests
)

$ErrorActionPreference = 'Stop'
Write-Host 'Running pre-commit checks (fmt, clippy, tests)'

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  Write-Error 'cargo not found in PATH'
  exit 1
}

Write-Host 'Formatting check...'
cargo fmt --all -- --check

Write-Host 'Clippy (deny warnings)...'
cargo clippy -- -D warnings

if (-not $NoTests) {
  Write-Host 'Tests...'
  cargo test -q -- --nocapture
}

Write-Host 'OK'
