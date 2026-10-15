@echo off
setlocal EnableExtensions
rem Compile-only fixture; do not register COM, change WSearch or open native UI.
set "VSROOT="
for %%V in ("%ProgramFiles%\Microsoft Visual Studio\2022\Enterprise" "%ProgramFiles%\Microsoft Visual Studio\2022\Community" "%ProgramFiles%\Microsoft Visual Studio\2022\Professional" "%ProgramFiles%\Microsoft Visual Studio\2022\BuildTools" "%ProgramFiles(x86)%\Microsoft Visual Studio\2022\BuildTools") do if not defined VSROOT if exist "%%~V\VC\Auxiliary\Build\vcvars64.bat" set "VSROOT=%%~V"
if not defined VSROOT (
  echo NATIVE_IFILTER_BUILD_BLOCKED_NO_MSVC
  exit /b 2
)
call "%VSROOT%\VC\Auxiliary\Build\vcvars64.bat" >nul
if errorlevel 1 exit /b 3
cd /d "%~dp0"
if not exist "out" mkdir "out"
cl /nologo /std:c++17 /utf-8 /EHsc /W4 /WX /LD /Fo"out\fixture_filter.obj" /Fe"out\fixture_filter.dll" fixture_filter.cpp ole32.lib uuid.lib /link /DEF:fixture_exports.def
if errorlevel 1 exit /b 4
cl /nologo /std:c++17 /utf-8 /EHsc /W4 /WX /Fo"out\fixture_test.obj" /Fe"out\fixture_test.exe" fixture_test.cpp ole32.lib uuid.lib
if errorlevel 1 exit /b 5
out\fixture_test.exe "%CD%\out\fixture_filter.dll"
if errorlevel 1 exit /b 6
echo NATIVE_IFILTER_BUILD_AND_LOCAL_COM_TEST_PASS
exit /b 0
