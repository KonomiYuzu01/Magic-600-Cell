@echo off
setlocal
cd /d "%~dp0.." || exit /b 1
if not exist build\tmp mkdir build\tmp
set "TMP=%CD%\build\tmp"
set "TEMP=%TMP%"
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if not exist "%VSWHERE%" (echo vswhere unavailable & exit /b 1)
for /f "usebackq delims=" %%i in (`call "%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSROOT=%%i"
if not defined VSROOT (echo MSVC x64 unavailable & exit /b 1)
call "%VSROOT%\VC\Auxiliary\Build\vcvars64.bat" >nul || exit /b 1
set "DXC=%WindowsSdkDir%bin\%WindowsSDKVersion%x64\dxc.exe"
if not exist "%DXC%" (echo Windows SDK DXC unavailable & exit /b 1)
"%DXC%" --version > build\dxc-version.txt 2>&1 || exit /b 1
"%DXC%" -T cs_6_2 -E main -HV 2021 -Gis -denorm preserve -O3 -Ges -WX generated.hlsl -Fo build\generated.dxil || exit /b 1
cl /std:c++17 /EHsc /W4 /O2 /utf-8 /Fo"build\main.obj" /Fe"build\chart_device.exe" device\main.cpp /link /INCREMENTAL:NO d3d12.lib dxgi.lib dxguid.lib || exit /b 1
exit /b 0
