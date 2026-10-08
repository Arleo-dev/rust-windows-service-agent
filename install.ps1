# Requires Administrator privileges
#Requires -RunAsAdministrator

$ServiceName = "FlamingoAgent"

Write-Host "[1/4] Building Rust Agent..." -ForegroundColor Cyan
Set-Location "$PSScriptRoot\agent-rust"
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "Rust build failed!" }

Write-Host "[2/4] Building C++ Child Binary..." -ForegroundColor Cyan
Set-Location "$PSScriptRoot\logger-child"
mkdir build -ErrorAction SilentlyContinue
Set-Location build
cmake ..
cmake --build . --config Release
if ($LASTEXITCODE -ne 0) { throw "C++ build failed!" }

Write-Host "Checking for running $ServiceName service..." -ForegroundColor Yellow
if (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue) {
    Write-Host "Stopping $ServiceName service..." -ForegroundColor Yellow
    Stop-Service -Name $ServiceName -Force

    $timeout = 0
    while ((Get-Process -Name "agent-rust" -ErrorAction SilentlyContinue) -and ($timeout -lt 5)) {
        Start-Sleep -Seconds 1
        $timeout++
    }

    if (Get-Process -Name "agent-rust" -ErrorAction SilentlyContinue) {
        Write-Host "Process still alive. Forcing termination..." -ForegroundColor Red
        Stop-Process -Name "agent-rust" -Force -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 1
    }
}

Write-Host "[3/4] Staging artifacts..." -ForegroundColor Cyan
$StageDir = "$PSScriptRoot\dist"
New-Item -ItemType Directory -Force -Path $StageDir | Out-Null
Copy-Item "$PSScriptRoot\agent-rust\target\release\agent-rust.exe" -Destination "$StageDir\agent-rust.exe" -Force
Copy-Item "$PSScriptRoot\logger-child\build\Release\logger_child.exe" -Destination "$StageDir\logger_child.exe" -Force

Write-Host "[4/4] Registering Windows Service..." -ForegroundColor Cyan
Set-Location $StageDir
.\agent-rust.exe --install

Write-Host "Installation completed successfully! Starting service..." -ForegroundColor Green
Start-Service -Name $ServiceName
Set-Location "$PSScriptRoot"