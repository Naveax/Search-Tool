// Guest-only CoCreateInstance test. Compiles on developer machines but
// is NEVER executed outside the admitted disposable Win11 ARM64 guest.
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <shlobj.h>
#include <shobjidl.h>
#include <cstdio>
#include <cwchar>
constexpr CLSID kPrivateClsid =
 {0x35dc8a17,0x4cd6,0x4a1b,{0xa0,0x0b,0x6f,0x0d,0x17,0xe4,0x45,0x52}};
int wmain(int argc,wchar_t** argv) {
    if (argc!=2 || std::wcscmp(argv[1],L"--private-hosted-guest")!=0) {
        std::fputs("PRIVATE_COM_ACTIVATION_GUEST_ONLY\n",stderr);
        return 3;
    }
    if (FAILED(CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED))) return 4;
    IShellFolder2* folder=nullptr;
    const HRESULT activation=CoCreateInstance(kPrivateClsid,nullptr,
        CLSCTX_INPROC_SERVER,IID_IShellFolder2,
        reinterpret_cast<void**>(&folder));
    if (FAILED(activation) || !folder) {
        std::printf("NATIVE_SEARCH_GUEST_SHELL_REGISTERED_ACTIVATION_HRESULT=0x%08lX\n",
            static_cast<unsigned long>(activation));
        CoUninitialize();
        return 5;
    }
    bool ok=true;
    IPersistFolder2* persist=nullptr;
    ok &= folder->QueryInterface(IID_IPersistFolder2,
        reinterpret_cast<void**>(&persist))==S_OK && persist;
    if (persist) {
        const USHORT root[]={4,0x1717,0};
        ok &= persist->Initialize(
            reinterpret_cast<PCIDLIST_ABSOLUTE>(root))==S_OK;
        persist->Release();
    }
    IEnumIDList* items=nullptr;
    ok &= folder->EnumObjects(nullptr,SHCONTF_NONFOLDERS,&items)==S_OK && items;
    if (items) {
        PITEMID_CHILD item=nullptr;
        ULONG count=0;
        ok &= items->Next(1,&item,&count)==S_OK &&
            item && count==1;
        if (item) CoTaskMemFree(item);
        items->Release();
    }
    folder->Release();
    CoUninitialize();
    if (ok) std::puts("NATIVE_SEARCH_GUEST_PRIVATE_SHELL_COM_ACTIVATION_PASS");
    return ok?0:6;
}