@echo off
title AI Dabing Studio - Spustenie
cd /d "%~dp0"

echo ==========================================================
echo        AI Dabing Studio (Slovencina -> Cinstina)
echo ==========================================================
echo.

where npm >nul 2>nul
if %errorlevel% neq 0 (
    echo [CHYBA] Program 'npm' nebol najdeny!
    echo Prosim spustite najskor: PowerShell (Spravca) -> .\install_prerequisites.ps1
    echo Alebo si nainstalujte Node.js z https://nodejs.org
    echo.
    pause
    exit /b 1
)

where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo [UPOZORNENIE] 'cargo' nebol najdeny v PATH. Skusam nacitat predvoleny .cargo/bin...
    set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
)

echo [1/2] Instalacia/kontrola frontendovych balickov (npm install)...
call npm install
if %errorlevel% neq 0 (
    echo [CHYBA] Zlyhala instalacia npm balickov.
    pause
    exit /b 1
)

echo.
echo [2/2] Spustanie Tauri desktopovej aplikacie (npm run tauri dev)...
call npm run tauri dev
pause
