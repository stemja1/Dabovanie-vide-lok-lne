# ==============================================================================
# AI Dabing Štúdio - Automatický inštalátor prerekvizít pre Windows 11
# ==============================================================================
# Tento skript automaticky nainštaluje a nastaví:
# 1. Node.js (LTS) a správcu balíčkov npm
# 2. Rust Toolchain (rustup, rustc, cargo)
# 3. Microsoft C++ Build Tools (potrebné pre kompiláciu Tauri v2)
# 4. WSL2 s distribúciou Ubuntu-24.04
# ==============================================================================

#Requires -RunAsAdministrator

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "   AI Dabing Studio - Inštalácia nástrojov (Windows 11)   " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host ""

# Funkcia na obnovenie systémových premenných PATH v bežiacom okne
function Refresh-EnvPath {
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
}

# 1. Kontrola a inštalácia Node.js + npm
Write-Host "[1/4] Kontrola Node.js a npm..." -ForegroundColor Yellow
$nodeInstalled = Get-Command node -ErrorAction SilentlyContinue
if ($nodeInstalled) {
    $nodeVer = node -v
    Write-Host "  -> Node.js je uz nainstalovany ($nodeVer)" -ForegroundColor Green
} else {
    Write-Host "  -> Node.js nie je nainstalovany. Instalujem Node.js LTS cez winget..." -ForegroundColor White
    winget install OpenJS.NodeJS.LTS --accept-package-agreements --accept-source-agreements --silent
    Refresh-EnvPath
}

# 2. Kontrola a inštalácia Rust Toolchain
Write-Host "[2/4] Kontrola Rust Toolchain (rustc / cargo)..." -ForegroundColor Yellow
$rustInstalled = Get-Command rustc -ErrorAction SilentlyContinue
if ($rustInstalled) {
    $rustVer = rustc --version
    Write-Host "  -> Rust je uz nainstalovany ($rustVer)" -ForegroundColor Green
} else {
    Write-Host "  -> Rust nie je najdeny. Stahujem a instalujem Rustup..." -ForegroundColor White
    $rustupExe = "$env:TEMP\rustup-init.exe"
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustupExe
    Start-Process -FilePath $rustupExe -ArgumentList "-y" -Wait -NoNewWindow
    Remove-Item $rustupExe -ErrorAction SilentlyContinue
    Refresh-EnvPath
}

# 3. Kontrola C++ Build Tools (MSVC)
Write-Host "[3/4] Kontrola Microsoft Visual C++ Build Tools..." -ForegroundColor Yellow
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$hasMsvc = $false
if (Test-Path $vswhere) {
    $msvcPath = & $vswhere -latest -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($msvcPath) {
        $hasMsvc = $true
        Write-Host "  -> Visual C++ Build Tools najdene v: $msvcPath" -ForegroundColor Green
    }
}

if (-not $hasMsvc) {
    Write-Host "  -> Instalujem Visual Studio Build Tools (C++)..." -ForegroundColor White
    winget install Microsoft.VisualStudio.2022.BuildTools --override "--passive --config ""https://aka.ms/vs/17/release/vs_buildtools.exe"" --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended" --accept-package-agreements --accept-source-agreements
}

# 4. Kontrola WSL2 a Ubuntu-24.04
Write-Host "[4/4] Kontrola WSL2 a Ubuntu-24.04..." -ForegroundColor Yellow
$wslCheck = wsl --status 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "  -> WSL2 je aktivne." -ForegroundColor Green
} else {
    Write-Host "  -> Pripravujem instalaciu WSL2 Ubuntu-24.04..." -ForegroundColor White
    wsl --install -d Ubuntu-24.04 --no-launch
}

Refresh-EnvPath

Write-Host ""
Write-Host "==========================================================" -ForegroundColor Green
Write-Host "   Vsetky zakladne nastroje boli uspesne nastavene!      " -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green
Write-Host ""
Write-Host "Dalsie kroky:" -ForegroundColor Cyan
Write-Host "1. Otvorte novy PowerShell (bez nutnosti spravcu) v priecinku projektu:"
Write-Host "   cd C:\Dabovanie-vide-lok-lne-main" -ForegroundColor White
Write-Host "2. Spustite aplikaciu prikazom:"
Write-Host "   npm install" -ForegroundColor White
Write-Host "   npm run tauri dev" -ForegroundColor White
Write-Host ""
