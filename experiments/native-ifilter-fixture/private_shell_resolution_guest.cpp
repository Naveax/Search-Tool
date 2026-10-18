// CI-only read-only query into the real Windows Shell parser.
// Never register COM, enumerate user files, show Explorer, or change WSearch.
// This binary is invoked only after strict disposable-guest preflight.
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <shlobj.h>
#include <shobjidl.h>
#include <cstdio>
#include <cwchar>
constexpr wchar_t kUrl[]=L"stfixture-proto-test://root/document";
constexpr wchar_t kAlternateUrl[]=L"stfixture-proto-test://root/document/";
int wmain(int argc,wchar_t** argv) {
    if (argc!=2 || std::wcscmp(argv[1],L"--guest-private-shell-observation")!=0) {
        std::fputs("PRIVATE_SHELL_URI_OBSERVATION_REQUIRES_GUEST_CALLER\n",stderr);
        return 3;
    }
    const HRESULT initialized=CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED);
    if (FAILED(initialized)) {
        std::puts("NATIVE_SEARCH_HOSTED_PRIVATE_SHELL_URI_CATEGORY=COM_UNAVAILABLE");
        return 4;
    }
    IShellItem* item=nullptr;
    const HRESULT hr=SHCreateItemFromParsingName(kUrl,nullptr,IID_IShellItem,
        reinterpret_cast<void**>(&item));
    if (FAILED(hr)) {
        std::puts("NATIVE_SEARCH_HOSTED_PRIVATE_SHELL_URI_CATEGORY=UNRESOLVED");
        std::printf("NATIVE_SEARCH_HOSTED_PRIVATE_SHELL_URI_HRESULT=0x%08lX\n",
                    static_cast<unsigned long>(hr));
        if (item) item->Release();
        CoUninitialize();
        return 0;
    }
    if (!item) {
        std::puts("NATIVE_SEARCH_HOSTED_PRIVATE_SHELL_URI_CATEGORY=NULL_SHELL_ITEM");
        CoUninitialize();
        return 5;
    }
    PWSTR observed=nullptr;
    const HRESULT pathHr=item->GetDisplayName(SIGDN_DESKTOPABSOLUTEPARSING,&observed);
    const bool correctPath=SUCCEEDED(pathHr) && observed &&
        (std::wcscmp(observed,kUrl)==0 ||
         std::wcscmp(observed,kAlternateUrl)==0);
    if (observed) CoTaskMemFree(observed);
    item->Release();
    CoUninitialize();
    if (!correctPath) {
        // A successfully created Shell object is NOT proof of matching
        // indexed-item identity. Negative category, not acceptance.
        // Never print returned names or any foreign Shell identity.
        std::puts("NATIVE_SEARCH_HOSTED_PRIVATE_SHELL_URI_CATEGORY=NONMATCHING_PARSING_NAME");
        std::printf("NATIVE_SEARCH_HOSTED_PRIVATE_SHELL_URI_HRESULT=0x%08lX\n",
                    static_cast<unsigned long>(pathHr));
        return 0;
    }
    // A generic Shell URI parser might produce IShellItem for literally
    // any URI. Require a negative-control observation before treating
    // exact parsing identity as evidence of a registered data source.
    // This control is a fixed, deliberately UNINDEXED synthetic URL.
    constexpr wchar_t kUnindexedControl[] =
        L"stfixture-proto-test://root/not-indexed-shell-control";
    IShellItem* control=nullptr;
    const HRESULT controlHr=SHCreateItemFromParsingName(kUnindexedControl,
        nullptr,IID_IShellItem,reinterpret_cast<void**>(&control));
    const bool parserAcceptsControl=SUCCEEDED(controlHr) && control;
    if (control) control->Release();
    if (SUCCEEDED(controlHr) && !parserAcceptsControl) {
        std::puts("NATIVE_SEARCH_HOSTED_PRIVATE_SHELL_URI_CATEGORY=CONTROL_NULL_SHELL_ITEM");
        return 7; // Fail closed on a malformed Shell contract.
    }
    std::puts(parserAcceptsControl
        ? "NATIVE_SEARCH_HOSTED_PRIVATE_SHELL_URI_CATEGORY=GENERIC_URI_PARSER_ACCEPTS_UNINDEXED_CONTROL"
        : "NATIVE_SEARCH_HOSTED_PRIVATE_SHELL_URI_CATEGORY=PRIVATE_URI_RESOLVED_CONTROL_REJECTED");
    std::puts("NATIVE_SEARCH_HOSTED_PRIVATE_SHELL_URI_HRESULT=0x00000000");
    return 0;
}