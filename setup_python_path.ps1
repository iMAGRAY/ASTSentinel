$pythonBase = 'C:\Users\1\AppData\Local\Programs\Python\Python313'
$pythonScripts = 'C:\Users\1\AppData\Local\Programs\Python\Python313\Scripts'

function Normalize([string]$p){
    ($p.Trim().TrimEnd('\')).ToLowerInvariant()
}

function Reorder-Path([string]$path){
    $parts=@()
    $winApps=@()
    $seen=[Collections.Generic.HashSet[string]]::new()
    
    foreach($raw in ($path -split ';')){
        $p=$raw.Trim()
        if(!$p){continue}
        $n=Normalize $p
        if(!$n -or $seen.Contains($n)){continue}
        $null=$seen.Add($n)
        if($n -like '*\microsoft\windowsapps*'){
            $winApps+=$p
        }else{
            $parts+=$p
        }
    }
    
    $prepend=@()
    if(!$seen.Contains((Normalize $pythonBase))){
        $prepend+=$pythonBase
    }
    if(!$seen.Contains((Normalize $pythonScripts))){
        $prepend+=$pythonScripts
    }
    
    (@($prepend)+$parts+$winApps) -join ';'
}

# Update user PATH
$userKey = 'HKCU:\Environment'
$userPath = (Get-ItemProperty $userKey -ErrorAction SilentlyContinue).Path

Write-Host "Current User PATH:" -ForegroundColor Cyan
Write-Host $userPath
Write-Host ""

$newUserPath = Reorder-Path $userPath

if($newUserPath -ne $userPath){
    Set-ItemProperty -Path $userKey -Name Path -Value $newUserPath
    Write-Host "User PATH updated successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "New User PATH:" -ForegroundColor Cyan
    Write-Host $newUserPath
}else{
    Write-Host "User PATH already has Python 3.13 properly configured." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Python 3.13 paths are now prioritized in your PATH environment variable." -ForegroundColor Green
Write-Host "You may need to restart your terminal for changes to take effect." -ForegroundColor Yellow