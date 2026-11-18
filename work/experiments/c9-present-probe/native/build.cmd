@echo off
setlocal
rem Builds c9pp.dll and c9pp_control.exe. Usage: build.cmd [BUILD_DIR]
rem Default build dir: <repo>\work\build\c9-present-probe (never %TEMP%: Smart App Control
rem refuses fresh unsigned binaries there). Set M600_C9PP_NMAKE=1 to force the NMake generator.
set "HERE=%~dp0"
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
set "PATH=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer;%PATH%"
set "VSROOT="
for /f "usebackq delims=" %%i in (`call "%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSROOT=%%i"
if not defined VSROOT (echo Visual Studio C++ x64 tools not found & exit /b 1)
call "%VSROOT%\VC\Auxiliary\Build\vcvars64.bat" >nul || exit /b 1
set "PATH=%VSROOT%\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin;%VSROOT%\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja;%PATH%"
set "CMAKE=cmake.exe"
set "NINJA=ninja.exe"
if "%~1"=="" (set "BUILD=%HERE%..\..\..\..\work\build\c9-present-probe") else (set "BUILD=%~1")
for %%b in ("%BUILD%") do set "BUILD=%%~fb"
cl 2>&1 | findstr /c:"Compiler Version"
echo Windows SDK %WindowsSDKVersion%
"%CMAKE%" --version || exit /b 1
if "%M600_C9PP_NMAKE%"=="1" goto nmake
where /q "%NINJA%" || goto nmake
"%NINJA%" --version || exit /b 1
"%CMAKE%" -S "%HERE%." -B "%BUILD%" -G Ninja -DCMAKE_MAKE_PROGRAM="%NINJA%" -DCMAKE_BUILD_TYPE=Release -DCMAKE_TRY_COMPILE_CONFIGURATION=Release || exit /b 1
goto compile
:nmake
echo Using the NMake Makefiles generator
nmake /? 2>&1 | findstr /c:"Version"
"%CMAKE%" -S "%HERE%." -B "%BUILD%" -G "NMake Makefiles" -DCMAKE_BUILD_TYPE=Release -DCMAKE_TRY_COMPILE_CONFIGURATION=Release || exit /b 1
:compile
"%CMAKE%" --build "%BUILD%" || exit /b 1
echo Built "%BUILD%\c9pp.dll" and "%BUILD%\c9pp_control.exe"
