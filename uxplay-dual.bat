@echo off
set PATH=C:\msys64\ucrt64\bin;%PATH%
start "Kagami-1" /min cmd /c ""%~dp0build\uxplay.exe" -n Kagami -nohold -vsync no -fps 60 -d > "%~dp0kagami1.log" 2>&1"
start "Kagami-2" /min cmd /c ""%~dp0build\uxplay.exe" -n Kagami2 -nohold -vsync no -fps 60 -m -d > "%~dp0kagami2.log" 2>&1"
timeout /t 6 /nobreak >nul
py -3.13 -u "%~dp0reflector.py"
taskkill /im uxplay.exe /f >nul 2>&1
