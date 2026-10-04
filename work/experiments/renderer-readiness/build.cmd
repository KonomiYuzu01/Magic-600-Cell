@echo off
rem Build the readiness program with the MSVC x64 toolset, CMake and Ninja from the renderer-spike venv.
setlocal
set "HERE=%~dp0"
set "VENV=%HERE%..\..\..\tools\.venv\renderer-spike\Scripts"
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
for /f "usebackq delims=" %%i in (`call "%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSROOT=%%i"
if not defined VSROOT (echo Visual Studio C++ x64 tools not found & exit /b 1)
call "%VSROOT%\VC\Auxiliary\Build\vcvars64.bat" >nul || exit /b 1
"%VENV%\cmake.exe" -S "%HERE%." -B "%HERE%build" -G Ninja -DCMAKE_MAKE_PROGRAM="%VENV%\ninja.exe" -DCMAKE_BUILD_TYPE=Release || exit /b 1
"%VENV%\cmake.exe" --build "%HERE%build" || exit /b 1
