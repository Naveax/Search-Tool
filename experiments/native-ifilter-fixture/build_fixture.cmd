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
cl /nologo /std:c++17 /utf-8 /EHsc /W4 /WX /LD /Fo"out\fixture_filter.obj" /Fe"out\fixture_filter.dll" fixture_filter.cpp ole32.lib uuid.lib advapi32.lib /link /DEF:fixture_exports.def
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
rem Compile registration smoke but NEVER execute it on developer PCs. Only
rem dedicated GitHub ephemeral guest test may temporarily register this CLSID.
cl /nologo /std:c++17 /utf-8 /EHsc /W4 /WX /Fo"out\registered_protocol_smoke.obj" /Fe"out\registered_protocol_smoke.exe" registered_protocol_smoke.cpp ole32.lib uuid.lib advapi32.lib
if errorlevel 1 exit /b 12
rem Read-only Windows Search Crawl Scope Manager binary. Compiled here,
rem executed separately only in the Win11 desktop guest preflight CI step.
cl /nologo /std:c++17 /utf-8 /EHsc /W4 /WX /Fo"out\crawl_scope_readonly.obj" /Fe"out\crawl_scope_readonly.exe" crawl_scope_readonly.cpp ole32.lib uuid.lib
if errorlevel 1 exit /b 13
rem Restricted private CSM guest mutator. NEVER execute on developer PC.
cl /nologo /std:c++17 /utf-8 /EHsc /W4 /WX /Fo"out\crawl_scope_guest_trial.obj" /Fe"out\crawl_scope_guest_trial.exe" crawl_scope_guest_trial.cpp ole32.lib uuid.lib advapi32.lib
if errorlevel 1 exit /b 14
rem Callback selftest filters out unrelated DocIDs and checks the three
rem phases. No Windows Search service, registry or index access is made.
cl /nologo /std:c++17 /utf-8 /EHsc /W4 /WX /Fo"out\private_inline_status_test.obj" /Fe"out\private_inline_status_test.exe" private_inline_status_test.cpp ole32.lib uuid.lib
if errorlevel 1 exit /b 15
out\private_inline_status_test.exe
if errorlevel 1 exit /b 16
rem Pre-registration Shell namespace contract: in-process private PIDL,
rem enum, parsing and attribute semantics. No Shell registration or Explorer UI.
cl /nologo /std:c++17 /utf-8 /EHsc /W4 /WX /Fo"out\private_shell_namespace_test.obj" /Fe"out\private_shell_namespace_test.exe" private_shell_namespace_test.cpp ole32.lib oleaut32.lib uuid.lib shell32.lib
if errorlevel 1 exit /b 17
out\private_shell_namespace_test.exe
if errorlevel 1 exit /b 18
rem Compile guest-only Shell URI parser probe but do NOT execute it on any
rem developer machine. It runs solely inside the isolated ARM64 guest script.
cl /nologo /std:c++17 /utf-8 /EHsc /W4 /WX /Fo"out\private_shell_resolution_guest.obj" /Fe"out\private_shell_resolution_guest.exe" private_shell_resolution_guest.cpp ole32.lib uuid.lib shell32.lib
if errorlevel 1 exit /b 19
echo NATIVE_IFILTER_BUILD_AND_LOCAL_COM_TEST_PASS
exit /b 0

:missing_toolchain
echo NATIVE_IFILTER_BUILD_BLOCKED_NO_VCVARS_USE_VSWHERE_WRAPPER
exit /b 2
