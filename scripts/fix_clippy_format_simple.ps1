#!/usr/bin/env pwsh
Set-StrictMode -Version Latest
Write-Output 'Запуск простого фикса format! (только тривиальные случаи)'
$files = Get-ChildItem -Path src -Filter *.rs -Recurse -File
foreach($f in $files){
    Write-Output "Processing $($f.FullName)"
    $orig = Get-Content $f.FullName -Raw
    $new = $orig

    # pattern: format!("... {} ...", var)
    $new = [regex]::Replace($new, 'format!\(\s*"([^"]*?)\"\s*,\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)', 'format!("$1{$2}")')

    # pattern: format!("... {} {} ...", a, b) -> format!("... {a} {b} ...")
    # handle two args
    $new = [regex]::Replace($new, 'format!\(\s*"([^"]*?)\{\}\s*\{\}\s*([^"]*?)"\s*,\s*([A-Za-z_][A-Za-z0-9_]*)\s*,\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)', 'format!("$1{$3}$2{$4}")')

    # pattern with single arg and simple format spec: format!("{:4}", x) -> format!("{x:4}")
    $new = [regex]::Replace($new, 'format!\(\s*"\{:(.*?)\}"\s*,\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)', 'format!("{$2:$1}")')

    if($new -ne $orig){
        Copy-Item $f.FullName ($f.FullName + '.bak_clippy_simple') -Force
        Set-Content -Path $f.FullName -Value $new -NoNewline
        Write-Output "Modified: $($f.FullName) -> backup created"
    } else {
        Write-Output "No changes: $($f.FullName)"
    }
}
Write-Output 'Done.'

