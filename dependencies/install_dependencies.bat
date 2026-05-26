@echo off
setlocal

:: =========================================================
:: FirmGen: ESP-IDF Pre-requisite Installer
:: Double-click this script to set up ESP-IDF and Toolchains
:: =========================================================

echo ==============================================
echo  FirmGen ESP-IDF Dependency Installer
echo ==============================================
echo.

:: -------------------------
:: CONFIG
:: -------------------------
set "ESP_VERSION=v5.5"
set "ESP_ROOT=C:\Espressif"
set "INSTALLER_URL=https://dl.espressif.com/dl/esp-idf/esp-idf-tools-setup-online-3.0.exe"
set "INSTALLER_EXE=%TEMP%\esp-idf-tools-setup-online.exe"

:: -------------------------
:: ADMIN CHECK
:: -------------------------
net session >nul 2>&1
if errorlevel 1 (
    echo [ERROR] This installer requires Administrator privileges.
    echo Please Right-Click this file and select "Run as Administrator".
    echo.
    pause
    exit /b 1
)

echo [1/3] Preparing workspace down in %ESP_ROOT%...
if not exist "%ESP_ROOT%" mkdir "%ESP_ROOT%"

echo.
echo [2/3] Downloading Official ESP-IDF Tools Setup...
echo (This might take a moment, please wait...)
curl.exe -L -o "%INSTALLER_EXE%" "%INSTALLER_URL%"

if not exist "%INSTALLER_EXE%" (
    echo.
    echo [ERROR] Failed to download the official ESP-IDF installer.
    echo Please check your internet connection.
    echo.
    pause
    exit /b 1
)

echo.
echo [3/3] Installing ESP-IDF %ESP_VERSION% components...
echo A setup window will appear to show you the installation progress.
echo Please do not close it until it finishes!
echo.

:: /SILENT shows a progress bar but skips all questions/prompts
:: /SP- Skips the "This will install" prompt
:: /IDFDIR Enforces the directory
:: /IDFVERSION Enforces the exact ESP version you need
start /wait "" "%INSTALLER_EXE%" /SILENT /SP- /IDFDIR="%ESP_ROOT%\esp-idf-release-%ESP_VERSION%" /IDFVERSION="%ESP_VERSION%"

if errorlevel 1 (
    echo.
    echo [ERROR] The ESP-IDF installation was cancelled or failed.
    echo.
    pause
    exit /b 1
)

echo.
echo ==============================================
echo  [SUCCESS] All dependencies have been installed!
echo  You may now launch the FirmGen application.
echo ==============================================
echo.
pause
exit /b 0
