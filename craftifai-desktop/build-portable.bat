@echo off
setlocal
cd /d "%~dp0"
echo Building GUI then Windows portable exe...
call npm run dist:win-portable
if errorlevel 1 exit /b 1
echo.
echo Output: dist-packages\CraftifAI-ESP32-Agent-*-portable.exe
endlocal
