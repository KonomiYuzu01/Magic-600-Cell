@echo off
setlocal
set "HERE=%~dp0"
for %%i in ("%HERE%..\..\..") do set "REPO=%%~fi"
set "QT=%REPO%\tools\qt\6.10.3\msvc2022_64"
if not exist "%QT%\lib\cmake\Qt6\Qt6Config.cmake" (echo Qt 6.10.3 msvc2022_64 directory is missing & exit /b 1)
set "VENV=%REPO%\tools\.venv\renderer-spike\Scripts"
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
set "VSROOT="
for /f "usebackq delims=" %%i in (`call "%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSROOT=%%i"
if not defined VSROOT (echo Visual Studio C++ x64 tools not found & exit /b 1)
call "%VSROOT%\VC\Auxiliary\Build\vcvars64.bat" >nul || exit /b 1
set "PATH=%VSROOT%\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin;%VSROOT%\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja;%PATH%"
set "CMAKE=cmake.exe"
set "NINJA=ninja.exe"
if exist "%VENV%\cmake.exe" set "CMAKE=%VENV%\cmake.exe"
if exist "%VENV%\ninja.exe" set "NINJA=%VENV%\ninja.exe"
if "%~1"=="" (set "BUILD=%REPO%\work\sdb\manual\app") else (set "BUILD=%~f1")
cl 2>&1 | findstr /c:"Compiler Version"
"%CMAKE%" --version || exit /b 1
"%NINJA%" --version || exit /b 1
"%CMAKE%" -S "%HERE%app" -B "%BUILD%" -G Ninja -DCMAKE_MAKE_PROGRAM="%NINJA%" -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH="%QT%" -DCMAKE_TRY_COMPILE_CONFIGURATION=Release || exit /b 1
"%CMAKE%" --build "%BUILD%" --target sd_smoke sd_code_layout_test sd_module_observer_test sd_module_probe || exit /b 1
echo Built sd_smoke.exe, sd_code_layout_test.exe, sd_module_observer_test.exe and sd_module_probe.dll
