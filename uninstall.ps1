# Requires Administrator privileges
#Requires -RunAsAdministrator

$ServiceName = "FlamingoAgent"

Write-Host "[1/3] Checking service status..." -ForegroundColor Cyan
if (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue) {
    Write-Host "Stopping '$ServiceName' service..." -ForegroundColor Yellow
    Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue

    $timeout = 0
    while ((Get-Process -Name "agent-rust" -ErrorAction SilentlyContinue) -and ($timeout -lt 5)) {
        Start-Sleep -Seconds 1
        $timeout++
    }

    if (Get-Process -Name "agent-rust" -ErrorAction SilentlyContinue) {
        Write-Host "Force terminating remaining agent process..." -ForegroundColor Red
        Stop-Process -Name "agent-rust" -Force -ErrorAction SilentlyContinue
    }

    Write-Host "[2/3] Removing service from Windows SCM..." -ForegroundColor Cyan
    sc.exe delete $ServiceName | Out-Null
    Write-Host "Service '$ServiceName' uninstalled successfully." -ForegroundColor Green
} else {
    Write-Host "Service '$ServiceName' is not registered." -ForegroundColor Yellow
}

Write-Host "[3/3] Cleaning up staged files..." -ForegroundColor Cyan
$StageDir = "$PSScriptRoot\dist"
if (Test-Path $StageDir) {
    Remove-Item -Path $StageDir -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "Removed dist/ folder." -ForegroundColor Green
}

Write-Host "Uninstallation completed successfully!" -ForegroundColor Green