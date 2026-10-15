@echo off
setlocal EnableExtensions
rem Compile-only fixture; do not register COM, change WSearch or open native UI.
if not defined SEARCHTOOL_VCVARS_PATH goto :missing_toolchain
if not exist "%SEARCHTOOL_VCVARS_PATH%" goto :missing_toolchain
call "%SEARCHTOOL_VCVARS_PATH%" >nul
if errorlevel 1 exit /b 3
if /I "%SEARCHTOOL_EXPECT_ARCH%"=="arm64" if /I not "%VSCMD_ARG_TGT_ARCH%"=="arm64" exit /b 10
if /I "%SEARCHTOOL_EXPECT_ARCH%"=="x64" if /I not "%VSCMD_ARG_TGT_ARCH%"=="x64" exit /b 11
cd /d "%~dp0"
if not exist "out" mkdir "out"
cl /nologo /std:c++17 /utf-8 /EHsc /W4 /WX /LD /Fo"out\fixture_filter.obj" /Fe"out\fixture_filter.dll" fixture_filter.cpp ole32.lib uuid.lib /link /DEF:fixture_exports.def
if errorlevel 1 exit /b 4
cl /nologo /std:c++17 /utf-8 /EHsc /W4 /WX /Fo"out\fixture_test.obj" /Fe"out\fixture_test.exe" fixture_test.cpp ole32.lib uuid.lib
if errorlevel 1 exit /b 5
out\fixture_test.exe "%CD%\out\fixture_filter.dll"
if errorlevel 1 exit /b 6
rem Separate private ISearchProtocol + IUrlAccessor synthetic URL factory.
cl /nologo /std:c++17 /utf-8 /EHsc /W4 /WX /LD /Fo"out\protocol_fixture.obj" /Fe"out\protocol_fixture.dll" protocol_fixture.cpp ole32.lib uuid.lib advapi32.lib /link /DEF:protocol_exports.def
if errorlevel 1 exit /b 7
cl /nologo /std:c++17 /utf-8 /EHsc /W4 /WX /Fo"out\protocol_test.obj" /Fe"out\protocol_test.exe" protocol_test.cpp ole32.lib uuid.lib advapi32.lib
if errorlevel 1 exit /b 8
out\protocol_test.exe "%CD%\out\protocol_fixture.dll" "%CD%\out\fixture_filter.dll"
if errorlevel 1 exit /b 9
echo NATIVE_IFILTER_BUILD_AND_LOCAL_COM_TEST_PASS
exit /b 0

:missing_toolchain
echo NATIVE_IFILTER_BUILD_BLOCKED_NO_VCVARS_USE_VSWHERE_WRAPPER
exit /b 2
