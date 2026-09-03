@echo off
setlocal enabledelayedexpansion
title AI Dabing Studio - Spustenie aplikacie
cd /d "%~dp0"

echo ==============================================================================
echo        AI Dabing Studio (Slovencina -> Cinstina) - Start
echo ==============================================================================
echo.

:: 1. Automaticke doplnenie standardnych ciest (Node.js, Rust Cargo, Git) do PATH
if exist "C:\Program Files\nodejs" (
    set "PATH=C:\Program Files\nodejs;!PATH!"
)
if exist "%LOCALAPPDATA%\Programs\nodejs" (
    set "PATH=%LOCALAPPDATA%\Programs\nodejs;!PATH!"
)
if exist "%APPDATA%\npm" (
    set "PATH=%APPDATA%\npm;!PATH!"
)
if exist "%USERPROFILE%\.cargo\bin" (
    set "PATH=%USERPROFILE%\.cargo\bin;!PATH!"
)
if exist "C:\Program Files\Git\cmd" (
    set "PATH=C:\Program Files\Git\cmd;!PATH!"
)

:: 2. Kontrola pritomnosti Node.js a npm
echo [1/3] Kontrola Node.js a npm...
where npm >nul 2>nul
if %errorlevel% neq 0 (
    echo.
    echo ==============================================================================
    echo [CHYBA] Nastroj 'npm' (Node.js) nebol najdeny!
    echo ==============================================================================
    echo Na vasom pocitaci chyba Node.js alebo este nebol dokonceny instalator.
    echo.
    echo RIESENIE:
    echo 1. Spustite PowerShell ako Spravca a zadajte:
    echo    cd "%~dp0"
    echo    powershell -ExecutionPolicy Bypass -File .\install_prerequisites.ps1
    echo 2. Alebo si stiahnite a nainstalujte Node.js LTS z: https://nodejs.org
    echo ==============================================================================
    echo.
    pause
    exit /b 1
)
echo    -> Node.js a npm su pripravene.

:: 3. Kontrola pritomnosti Rust (cargo)
echo [2/3] Kontrola Rust kompilatora (cargo)...
where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo.
    echo ==============================================================================
    echo [UPOZORNENIE] 'cargo' (Rust) nebol najdeny v systemovej ceste PATH.
    echo Pokusam sa pouzit: %USERPROFILE%\.cargo\bin\cargo.exe
    echo ==============================================================================
    if not exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
        echo.
        echo [CHYBA] Rust nie je nainstalovany!
        echo Prosim spustite inštalátor: powershell -ExecutionPolicy Bypass -File .\install_prerequisites.ps1
        echo.
        pause
        exit /b 1
    )
)
echo    -> Rust toolchain je pripraveny.

:: 4. Kontrola a instalacia balickov
echo.
echo [3/3] Priprava kniznic a spustanie Tauri desktopoveho okna...
echo       (Prve spustenie moze trvat 1-2 minuty kvoli kompilacii Rustu)
echo.

if not exist "node_modules\@tauri-apps\cli" (
    echo [Info] Instalujem potrebne balicky (npm install)...
    call npm install
    if %errorlevel% neq 0 (
        echo.
        echo [CHYBA] 'npm install' skoncil s chybou.
        pause
        exit /b 1
    )
)

:: 5. Spustenie aplikacie cez npm run tauri dev (alebo npx fallback)
echo Spustam aplikaciu...
call npm run tauri dev
if %errorlevel% neq 0 (
    echo.
    echo [Info] Skusam spustenie cez priamy npx fallback...
    call npx tauri dev
)

if %errorlevel% neq 0 (
    echo.
    echo ==============================================================================
    echo [CHYBA] Aplikaciu sa nepodarilo spustit.
    echo Pozrite si chybovu hlasku vyssie.
    echo Ak chyba Visual C++ Build Tools, spustite:
    echo powershell -ExecutionPolicy Bypass -File .\install_prerequisites.ps1
    echo ==============================================================================
    echo.
    pause
    exit /b 1
)

pause
