@echo off
chcp 65001 >nul
title AI Dabing Studio - Inštalácia AI Modelov a ROCm vo WSL2

echo ==============================================================================
echo    AI Dabing Štúdio (Slovenčina -^> Čínština) - Inštalátor do WSL2 Ubuntu
echo ==============================================================================
echo.
echo Tento skript spustí inštaláciu priamo vnútri Linuxu (Ubuntu 24.04).
echo Uvidíte reálny priebeh sťahovania každého modelu s percentami.
echo.

wsl.exe -d Ubuntu-24.04 -u root -- bash -c "apt-get update -y && apt-get install -y dos2unix curl wget git"
wsl.exe -d Ubuntu-24.04 -- bash -c "mkdir -p ~/ai_dubbing_workspace/scripts; cp -ru /mnt/c/*/Dabovanie*/*/setup_wsl_environment.sh ~/ai_dubbing_workspace/scripts/ 2>/dev/null || cp -ru /mnt/c/*/*/setup_wsl_environment.sh ~/ai_dubbing_workspace/scripts/ 2>/dev/null || curl -sSL https://raw.githubusercontent.com/stemja1/Dabovanie-vide-lok-lne/main/scripts/setup_wsl_environment.sh -o ~/ai_dubbing_workspace/scripts/setup_wsl_environment.sh; dos2unix ~/ai_dubbing_workspace/scripts/setup_wsl_environment.sh 2>/dev/null; bash ~/ai_dubbing_workspace/scripts/setup_wsl_environment.sh"

echo.
echo ==============================================================================
echo Inštalácia dokončená. Teraz môžete otvoriť AI Dabing Štúdio a spustiť dabing.
echo ==============================================================================
echo.
pause
