@echo off
setlocal
set "HERE=%~dp0"
set "BUILD_DIR=%HERE%build"
if not "%~1"=="" set "BUILD_DIR=%~f1"
set "VENV=%HERE%..\..\..\tools\.venv\renderer-spike\Scripts"
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if not exist "%VSWHERE%" (echo Visual Studio vswhere.exe not found & exit /b 1)
for /f "usebackq delims=" %%i in (`call "%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSROOT=%%i"
if not defined VSROOT (echo Visual Studio C++ x64 tools not found & exit /b 1)
call "%VSROOT%\VC\Auxiliary\Build\vcvars64.bat" >nul || exit /b 1
set "CMAKE=cmake.exe"
set "NINJA=ninja.exe"
if exist "%VENV%\cmake.exe" set "CMAKE=%VENV%\cmake.exe"
if exist "%VENV%\ninja.exe" set "NINJA=%VENV%\ninja.exe"
set "DXC=%WindowsSdkDir%bin\%WindowsSDKVersion%x64\dxc.exe"
if not exist "%DXC%" (echo Windows SDK dxc.exe not found & exit /b 1)
echo CMake:
"%CMAKE%" --version || exit /b 1
echo Ninja:
"%NINJA%" --version || exit /b 1
echo MSVC:
cl 2>&1
echo DXC:
"%DXC%" --version || exit /b 1
if defined M600_WJ_SERIAL_NINJA goto serial_ninja
"%CMAKE%" -S "%HERE%." -B "%BUILD_DIR%" -G Ninja -DCMAKE_MAKE_PROGRAM="%NINJA%" -DDXC_EXECUTABLE="%DXC%" -DCMAKE_BUILD_TYPE=Release -DCMAKE_TRY_COMPILE_CONFIGURATION=Release || exit /b 1
"%CMAKE%" --build "%BUILD_DIR%" || exit /b 1
goto build_ok
:serial_ninja
rem Ninja child-output pipes hang under the restricted Windows token. Generate
rem the same graph, then execute its command list without Ninja's child pipes.
rem Actual compilation and linking below replace the compiler smoke test.
"%CMAKE%" -S "%HERE%." -B "%BUILD_DIR%" -G Ninja -DCMAKE_MAKE_PROGRAM="%NINJA%" -DDXC_EXECUTABLE="%DXC%" -DCMAKE_BUILD_TYPE=Release -DCMAKE_CXX_COMPILER_FORCED=TRUE || exit /b 1
python "%HERE%build_probe.py" "%BUILD_DIR%" "%NINJA%" || exit /b 1
:build_ok
echo build: ok
exit /b 0
