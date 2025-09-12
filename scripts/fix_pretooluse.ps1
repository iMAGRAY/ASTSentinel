param()
$path = 'src\bin\pretooluse.rs'
if(-not (Test-Path $path)) { Write-Error "File not found: $path"; exit 1 }
$lines = Get-Content $path
$start = 332
$end = 335
Write-Output "Backing up $path -> $path.bak_linefix"
Copy-Item $path ($path + '.bak_linefix') -Force
Write-Output "Original lines ($start..$end):"
$lines[$start..$end] | ForEach-Object { Write-Output "-> $_" }
$newBlock = @(
    '        if !seen_named.is_empty() {',
    '            issues.push(format!("Calls to `{}` still pass removed named params: {}",',
    '                fname,',
    '                seen_named.join(", ")',
    '            ));',
    '        }'
)
$newLines = @()
if($start -gt 0) { $newLines += $lines[0..($start-1)] }
$newLines += $newBlock
if($end -lt $lines.Count-1) { $newLines += $lines[($end+1)..($lines.Count-1)] }
Set-Content -Path $path -Value $newLines -NoNewline
Write-Output "Wrote modified file: $path"

