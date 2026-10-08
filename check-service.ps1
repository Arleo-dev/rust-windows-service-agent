# Requires Administrator privileges
#Requires -RunAsAdministrator

if (Get-Service -Name "FlamingoAgent" -ErrorAction SilentlyContinue) {
    Write-Host "FlamingoAgent Service is INSTALLED" -ForegroundColor Green
} else {
    Write-Host "FlamingoAgent Service is NOT installed" -ForegroundColor Red
}

$FilesToCheck = @(
    "$PSScriptRoot\dist\metrics.log",
    "$PSScriptRoot\dist\logger_child.exe"
)

Write-Host "--- Checking Security Descriptor / Access Control Lists ---" -ForegroundColor Cyan

foreach ($FilePath in $FilesToCheck) {
    if (Test-Path $FilePath) {
        Write-Host "Auditing permissions for: $FilePath" -ForegroundColor Yellow
        
        $Acl = Get-Acl -Path $FilePath
        $AccessRules = $Acl.Access
        
        $UnauthorizedRules = $AccessRules | Where-Object { 
            $_.IdentityReference -notmatch "SYSTEM" -and 
            $_.IdentityReference -notmatch "Administrators"
        }

        if ($UnauthorizedRules.Count -eq 0) {
            Write-Host "   [+] PASS: File access is strictly restricted to SYSTEM and Administrators!" -ForegroundColor Green
        } else {
            Write-Host "   [-] FAIL: Unrestricted/Unauthorized identities detected:" -ForegroundColor Red
            foreach ($Rule in $UnauthorizedRules) {
                Write-Host "      - $($Rule.IdentityReference) ($($Rule.FileSystemRights))" -ForegroundColor Red
            }
        }
    } else {
        Write-Host "File not found: $FilePath" -ForegroundColor DarkYellow
    }
}
