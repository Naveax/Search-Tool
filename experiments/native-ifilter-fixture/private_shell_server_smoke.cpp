// Registrationless OS LoadLibrary + real COM class factory smoke test.
// No registry writes, Explorer, SearchHost, WSearch or user-file access.
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <shlobj.h>
#include <shobjidl.h>
#include <cstdio>
using GetFactory = HRESULT (STDAPICALLTYPE*)(REFCLSID,REFIID,void**);
constexpr CLSID kPrivateClsid =
 {0x35dc8a17,0x4cd6,0x4a1b,{0xa0,0x0b,0x6f,0x0d,0x17,0xe4,0x45,0x52}};
int wmain(int argc,wchar_t** argv) {
    if (argc!=2) return 3;
    if (FAILED(CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED))) return 4;
    bool ok=true;
    HMODULE lib=LoadLibraryW(argv[1]);
    if (!lib) {CoUninitialize();return 5;}
    auto get=reinterpret_cast<GetFactory>(GetProcAddress(lib,"DllGetClassObject"));
    ok &= get!=nullptr;
    if (get) {
        CLSID foreign=kPrivateClsid;
        foreign.Data1 ^= 0x1;
        void* denied=reinterpret_cast<void*>(0x1);
        ok &= get(foreign,IID_IClassFactory,&denied)==
            CLASS_E_CLASSNOTAVAILABLE && !denied;
        denied=reinterpret_cast<void*>(0x1);
        ok &= get(kPrivateClsid,IID_IStream,&denied)==
            E_NOINTERFACE && !denied;
        IClassFactory* factory=nullptr;
        ok &= get(kPrivateClsid,IID_IClassFactory,
            reinterpret_cast<void**>(&factory))==S_OK && factory;
        if (factory) {
            void* aggregated=reinterpret_cast<void*>(0x1);
            ok &= factory->CreateInstance(
                reinterpret_cast<IUnknown*>(0x1),IID_IShellFolder,
                &aggregated)==CLASS_E_NOAGGREGATION && !aggregated;
            IShellFolder2* shell=nullptr;
            ok &= factory->CreateInstance(nullptr,IID_IShellFolder2,
                reinterpret_cast<void**>(&shell))==S_OK && shell;
            if (shell) {
                IPersistFolder2* persist=nullptr;
                ok &= shell->QueryInterface(IID_IPersistFolder2,
                    reinterpret_cast<void**>(&persist))==S_OK && persist;
                if (persist) {
                    const USHORT root[]={4,0x1717,0};
                    ok &= persist->Initialize(
                        reinterpret_cast<PCIDLIST_ABSOLUTE>(root))==S_OK;
                    PIDLIST_ABSOLUTE current=nullptr;
                    ok &= persist->GetCurFolder(&current)==S_OK && current;
                    if (current) CoTaskMemFree(current);
                    persist->Release();
                }
                IEnumIDList* enumeration=nullptr;
                ok &= shell->EnumObjects(nullptr,SHCONTF_NONFOLDERS,
                    &enumeration)==S_OK && enumeration;
                if (enumeration) {
                    PITEMID_CHILD item=nullptr;
                    ULONG count=0;
                    ok &= enumeration->Next(1,&item,&count)==S_OK &&
                        item && count==1;
                    if (item) CoTaskMemFree(item);
                    enumeration->Release();
                }
                shell->Release();
            }
            factory->Release();
        }
    }
    FreeLibrary(lib);
    CoUninitialize();
    if (ok) std::puts("PRIVATE_SHELL_COM_SERVER_FACTORY_NO_REGISTRATION_PASS");
    return ok?0:6;
}