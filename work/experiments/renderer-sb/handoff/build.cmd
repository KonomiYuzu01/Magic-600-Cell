@echo off
setlocal
set "HERE=%~dp0"
set "VENV=%HERE%..\..\..\..\tools\.venv\renderer-spike\Scripts"
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
set "PATH=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer;%PATH%"
set "VSROOT="
for /f "usebackq delims=" %%i in (`call "%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSROOT=%%i"
if not defined VSROOT (echo Visual Studio C++ x64 tools not found & exit /b 1)
call "%VSROOT%\VC\Auxiliary\Build\vcvars64.bat" >nul || exit /b 1
set "PATH=%VSROOT%\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin;%VSROOT%\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja;%PATH%"
set "CMAKE=cmake.exe"
set "NINJA=ninja.exe"
if exist "%VENV%\cmake.exe" set "CMAKE=%VENV%\cmake.exe"
if exist "%VENV%\ninja.exe" set "NINJA=%VENV%\ninja.exe"
if "%~1"=="" (set "BUILD=%TEMP%\m600-sb-handoff-build") else (set "BUILD=%~f1")
cl 2>&1 | findstr /c:"Compiler Version"
"%CMAKE%" --version || exit /b 1
"%NINJA%" --version || exit /b 1
if "%M600_HANDOFF_CPU_CHECK%"=="1" goto nmake_check
"%CMAKE%" -S "%HERE%." -B "%BUILD%" -G Ninja -DCMAKE_MAKE_PROGRAM="%NINJA%" -DCMAKE_BUILD_TYPE=Release -DCMAKE_TRY_COMPILE_CONFIGURATION=Release || exit /b 1
goto compile
:nmake_check
rem Ninja's subprocess completion stalls even for cmd /c echo in the Windows
rem implementation sandbox. The CPU acceptance check uses the installed NMake.
echo CPU acceptance: using NMake fallback
nmake /? 2>&1 | findstr /c:"Version"
"%CMAKE%" -S "%HERE%." -B "%BUILD%" -G "NMake Makefiles" -DCMAKE_BUILD_TYPE=Release -DCMAKE_TRY_COMPILE_CONFIGURATION=Release || exit /b 1
:compile
"%CMAKE%" --build "%BUILD%" || exit /b 1
echo Built "%BUILD%\sb_handoff.exe"
