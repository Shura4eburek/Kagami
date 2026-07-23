@echo off
set PATH=C:\msys64\ucrt64\bin;%PATH%
start "Kagami" /min "%~dp0build\uxplay.exe" -n Kagami -nohold -vsync no -fps 60 %*
timeout /t 5 /nobreak >nul
py -3.13 -u "%~dp0reflector.py"
taskkill /im uxplay.exe /f >nul 2>&1
