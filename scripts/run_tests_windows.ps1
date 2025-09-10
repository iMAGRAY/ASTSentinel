Param(
    [switch]$Fast
)

Write-Host "== Windows test runner =="
Write-Host "Rustc version:" -NoNewline; & rustc -V
Write-Host "Cargo version:" -NoNewline; & cargo -V

if ($LASTEXITCODE -ne 0) { throw "Rust toolchain not found. Please install Rust (rustup)." }

if ($Fast) {
    Write-Host "[1/1] cargo test (default features)"
    & cargo test --locked
    exit $LASTEXITCODE
}

Write-Host "[1/4] cargo build --locked"
& cargo build --locked
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "[2/4] cargo test --locked (default features)"
& cargo test --locked
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "[3/4] cargo test --features ast_fastpath --locked"
& cargo test --features ast_fastpath --locked
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "[4/4] cargo test --no-default-features --locked"
& cargo test --no-default-features --locked
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "All Windows tests passed."
