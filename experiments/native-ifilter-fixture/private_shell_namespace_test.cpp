// Registrationless, in-process Shell namespace contract experiment.
// ONE synthetic private item ONLY; never register an Explorer namespace
// extension, never launch explorer.exe, and never read user files.
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <shlobj.h>
#include <shobjidl.h>
#include <atomic>
#include <cstdio>
#include <cwchar>
#include <cstring>
#include <new>

namespace {
constexpr USHORT kPidlSize=4;
constexpr USHORT kMagic=0x5346;
constexpr wchar_t kItemName[]=L"document.stfixture";
constexpr wchar_t kItemUrl[]=L"stfixture-proto-test://root/document";
const CLSID kFixtureShellClsid =
 {0x35dc8a17,0x4cd6,0x4a1b,{0xa0,0x0b,0x6f,0x0d,0x17,0xe4,0x45,0x52}};

PITEMID_CHILD NewSyntheticPidl() {
    auto* data=static_cast<BYTE*>(CoTaskMemAlloc(6));
    if (!data) return nullptr;
    const USHORT encoded[]={kPidlSize,kMagic,0};
    std::memcpy(data,encoded,sizeof(encoded));
    return reinterpret_cast<PITEMID_CHILD>(data);
}
bool IsSyntheticPidl(PCUITEMID_CHILD item) {
    if (!item || item->mkid.cb != kPidlSize) return false;
    USHORT marker=0;
    std::memcpy(&marker,item->mkid.abID,sizeof(marker));
    const BYTE* raw=reinterpret_cast<const BYTE*>(item);
    USHORT terminator=1;
    std::memcpy(&terminator,raw+kPidlSize,sizeof(terminator));
    return marker==kMagic && terminator==0;
}
HRESULT ToStrret(PCWSTR name,STRRET* out) {
    if (!out) return E_POINTER;
    const size_t len=std::wcslen(name);
    auto* memory=static_cast<wchar_t*>(CoTaskMemAlloc((len+1)*sizeof(wchar_t)));
    if (!memory) return E_OUTOFMEMORY;
    std::memcpy(memory,name,(len+1)*sizeof(wchar_t));
    out->uType=STRRET_WSTR;
    out->pOleStr=memory;
    return S_OK;
}
class PrivateEnum final:public IEnumIDList {
public:
    explicit PrivateEnum(ULONG index=0):index_(index){}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id,void** out) override {
        if (!out) return E_POINTER;
        *out=nullptr;
        if (id!=IID_IUnknown && id!=IID_IEnumIDList) return E_NOINTERFACE;
        *out=static_cast<IEnumIDList*>(this);AddRef();return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override {return ++refs_;}
    ULONG STDMETHODCALLTYPE Release() override {
        ULONG r=--refs_;if (!r) delete this;return r;
    }
    HRESULT STDMETHODCALLTYPE Next(ULONG count,PITEMID_CHILD* output,ULONG* fetched) override {
        if (fetched) *fetched=0;
        if (!output || (count>1 && !fetched)) return E_INVALIDARG;
        for (ULONG i=0;i<count;++i) output[i]=nullptr;
        if (!count) return S_OK;
        if (index_!=0) return S_FALSE;
        auto* item=NewSyntheticPidl();
        if (!item) return E_OUTOFMEMORY;
        output[0]=item;index_=1;
        if (fetched) *fetched=1;
        return count==1 ? S_OK : S_FALSE;
    }
    HRESULT STDMETHODCALLTYPE Skip(ULONG count) override {
        if (!count) return S_OK;
        if (index_!=0) return S_FALSE;
        index_=1;
        return count==1 ? S_OK : S_FALSE;
    }
    HRESULT STDMETHODCALLTYPE Reset() override {index_=0;return S_OK;}
    HRESULT STDMETHODCALLTYPE Clone(IEnumIDList** out) override {
        if (!out) return E_POINTER;
        *out=new (std::nothrow) PrivateEnum(index_);
        return *out ? S_OK:E_OUTOFMEMORY;
    }
private:
    std::atomic<ULONG> refs_{1};
    ULONG index_=0;
};
class PrivateFolder final:public IShellFolder,public IPersistFolder {
public:
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id,void** out) override {
        if (!out) return E_POINTER;
        *out=nullptr;
        if (id==IID_IUnknown || id==IID_IShellFolder)
            *out=static_cast<IShellFolder*>(this);
        else if (id==IID_IPersist || id==IID_IPersistFolder)
            *out=static_cast<IPersistFolder*>(this);
        else return E_NOINTERFACE;
        AddRef();return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override {return ++refs_;}
    ULONG STDMETHODCALLTYPE Release() override {
        ULONG r=--refs_;if(!r) delete this;return r;
    }
    HRESULT STDMETHODCALLTYPE GetClassID(CLSID* out) override {
        if (!out) return E_POINTER;
        *out=kFixtureShellClsid;return S_OK;
    }
    HRESULT STDMETHODCALLTYPE Initialize(PCIDLIST_ABSOLUTE root) override {
        if (!root) return E_INVALIDARG;
        initialized_=true;return S_OK;
    }
    HRESULT STDMETHODCALLTYPE ParseDisplayName(HWND,IBindCtx*,LPWSTR name,ULONG* eaten,
                                               PIDLIST_RELATIVE* out,ULONG* attributes) override {
        if (eaten) *eaten=0;
        if (!out) return E_POINTER;
        *out=nullptr;
        if (!initialized_ || !name) return E_INVALIDARG;
        if (std::wcscmp(name,kItemName)!=0 && std::wcscmp(name,kItemUrl)!=0)
            return HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND);
        *out=NewSyntheticPidl();
        if (!*out) return E_OUTOFMEMORY;
        if (eaten) *eaten=static_cast<ULONG>(std::wcslen(name));
        if (attributes) *attributes &= SFGAO_STREAM|SFGAO_READONLY;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE EnumObjects(HWND,SHCONTF flags,IEnumIDList** out) override {
        if (!out) return E_POINTER;
        *out=nullptr;
        if (!initialized_) return E_UNEXPECTED;
        *out=new (std::nothrow) PrivateEnum((flags&SHCONTF_NONFOLDERS)?0:1);
        return *out?S_OK:E_OUTOFMEMORY;
    }
    HRESULT STDMETHODCALLTYPE BindToObject(PCUIDLIST_RELATIVE,IBindCtx*,REFIID,void** out) override {
        if (out) *out=nullptr;
        return E_NOTIMPL; // Leaf only: not an Explorer-ready Shell data source.
    }
    HRESULT STDMETHODCALLTYPE BindToStorage(PCUIDLIST_RELATIVE,IBindCtx*,REFIID,void** out) override {
        if (out) *out=nullptr;
        return E_NOTIMPL; // No user data access or arbitrary stream forwarding.
    }
    HRESULT STDMETHODCALLTYPE CompareIDs(LPARAM,PCUIDLIST_RELATIVE a,PCUIDLIST_RELATIVE b) override {
        if (!IsSyntheticPidl(a) || !IsSyntheticPidl(b)) return E_INVALIDARG;
        return MAKE_HRESULT(SEVERITY_SUCCESS,0,0);
    }
    HRESULT STDMETHODCALLTYPE CreateViewObject(HWND,REFIID,void** out) override {
        if (out) *out=nullptr;
        return E_NOTIMPL; // Cannot claim Explorer/UI acceptance.
    }
    HRESULT STDMETHODCALLTYPE GetAttributesOf(UINT count,PCUITEMID_CHILD_ARRAY items,
                                              SFGAOF* attributes) override {
        if (!attributes) return E_POINTER;
        if (!count || !items) return E_INVALIDARG;
        for (UINT i=0;i<count;++i) {
            if (!IsSyntheticPidl(items[i])) return E_INVALIDARG;
        }
        *attributes &= SFGAO_STREAM|SFGAO_READONLY;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE GetUIObjectOf(HWND,UINT,PCUITEMID_CHILD_ARRAY,
                                            REFIID,UINT*,void** out) override {
        if (out) *out=nullptr;
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE GetDisplayNameOf(PCUITEMID_CHILD item,SHGDNF flags,STRRET* out) override {
        if (!IsSyntheticPidl(item)) return E_INVALIDARG;
        return ToStrret((flags&SHGDN_FORPARSING)?kItemUrl:kItemName,out);
    }
    HRESULT STDMETHODCALLTYPE SetNameOf(HWND,PCUITEMID_CHILD,LPCWSTR,SHGDNF,PITEMID_CHILD* out) override {
        if (out) *out=nullptr;
        return E_ACCESSDENIED; // Never allow editing.
    }
private:
    std::atomic<ULONG> refs_{1};
    bool initialized_=false;
};
bool Check(bool yes,const char* step) {
    if (!yes) std::fprintf(stderr,"PRIVATE_SHELL_CONTRACT_FAIL_%s\n",step);
    return yes;
}
}
int main() {
    const HRESULT hr=CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED);
    if (FAILED(hr)) return 1;
    bool ok=true;
    auto* folder=new PrivateFolder();
    IShellFolder* shell=nullptr;
    IPersistFolder* persist=nullptr;
    ok &= Check(folder->QueryInterface(IID_IShellFolder,
        reinterpret_cast<void**>(&shell))==S_OK && shell,"QUERY_FOLDER");
    ok &= Check(folder->QueryInterface(IID_IPersistFolder,
        reinterpret_cast<void**>(&persist))==S_OK && persist,"QUERY_PERSIST");
    CLSID id{};
    ok &= Check(persist && persist->GetClassID(&id)==S_OK &&
        id==kFixtureShellClsid,"CLASS_ID");
    const USHORT root[]={4,0x1717,0};
    ok &= Check(persist && persist->Initialize(
        reinterpret_cast<PCIDLIST_ABSOLUTE>(root))==S_OK,"ROOT_INIT");
    IEnumIDList* enumeration=nullptr;
    ok &= Check(shell && shell->EnumObjects(nullptr,SHCONTF_NONFOLDERS,&enumeration)==S_OK &&
        enumeration,"ENUM");
    PITEMID_CHILD item=nullptr;
    ULONG count=0;
    ok &= Check(enumeration && enumeration->Next(1,&item,&count)==S_OK &&
        count==1 && IsSyntheticPidl(item),"ENUM_ITEM");
    PITEMID_CHILD second=nullptr;
    count=99;
    ok &= Check(enumeration->Next(1,&second,&count)==S_FALSE &&
        !second && count==0,"ENUM_END");
    IEnumIDList* exhaustedClone=nullptr;
    ok &= Check(enumeration->Clone(&exhaustedClone)==S_OK && exhaustedClone,
        "CLONE");
    count=99;
    ok &= Check(exhaustedClone && exhaustedClone->Next(1,&second,&count)==S_FALSE &&
        !second && count==0,"CLONED_ENUM_END");
    ok &= Check(exhaustedClone->Reset()==S_OK,"CLONE_RESET");
    PITEMID_CHILD cloneItem=nullptr;
    count=0;
    ok &= Check(exhaustedClone->Next(1,&cloneItem,&count)==S_OK &&
        count==1 && IsSyntheticPidl(cloneItem),"CLONE_RESET_ITEM");
    if (cloneItem) CoTaskMemFree(cloneItem);
    exhaustedClone->Release();
    IEnumIDList* folderOnly=nullptr;
    ok &= Check(shell->EnumObjects(nullptr,SHCONTF_FOLDERS,&folderOnly)==S_OK &&
        folderOnly,"FOLDER_FILTER");
    count=99;
    ok &= Check(folderOnly->Next(1,&second,&count)==S_FALSE &&
        !second && count==0,"NO_IMPERSONATED_SUBFOLDERS");
    folderOnly->Release();
    STRRET friendly{};
    ok &= Check(shell->GetDisplayNameOf(item,SHGDN_NORMAL,&friendly)==S_OK &&
        friendly.uType==STRRET_WSTR && std::wcscmp(friendly.pOleStr,kItemName)==0,
        "NAME");
    if (friendly.uType==STRRET_WSTR) CoTaskMemFree(friendly.pOleStr);
    STRRET parsing{};
    ok &= Check(shell->GetDisplayNameOf(item,SHGDN_FORPARSING,&parsing)==S_OK &&
        parsing.uType==STRRET_WSTR && std::wcscmp(parsing.pOleStr,kItemUrl)==0,
        "PARSE_NAME");
    if (parsing.uType==STRRET_WSTR) CoTaskMemFree(parsing.pOleStr);
    wchar_t name[]=L"document.stfixture";
    PIDLIST_RELATIVE parsed=nullptr;
    ULONG eaten=0;
    ok &= Check(shell->ParseDisplayName(nullptr,nullptr,name,&eaten,&parsed,nullptr)==S_OK &&
        IsSyntheticPidl(reinterpret_cast<PCUITEMID_CHILD>(parsed)) &&
        eaten==std::wcslen(name),"PARSE");
    wchar_t unknown[]=L"private-user-file.txt";
    PIDLIST_RELATIVE unknownResult=nullptr;
    ok &= Check(shell->ParseDisplayName(nullptr,nullptr,unknown,nullptr,
        &unknownResult,nullptr)==HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND) &&
        !unknownResult,"REJECT_UNKNOWN_USER_FILE");
    const USHORT wrong[]={4,0x1717,0};
    ok &= Check(shell->GetDisplayNameOf(
        reinterpret_cast<PCUITEMID_CHILD>(wrong),SHGDN_NORMAL,&friendly)
        ==E_INVALIDARG,"REJECT_NONFIXTURE_PIDL");
    SFGAOF flags=SFGAO_STREAM|SFGAO_READONLY|SFGAO_FILESYSTEM|SFGAO_FOLDER;
    PCUITEMID_CHILD one[]={item};
    ok &= Check(shell->GetAttributesOf(1,one,&flags)==S_OK &&
        (flags&SFGAO_STREAM)!=0 && !(flags&SFGAO_FILESYSTEM) &&
        !(flags&SFGAO_FOLDER),"ATTRIBUTES");
    void* view=nullptr;
    ok &= Check(shell->CreateViewObject(nullptr,IID_IUnknown,&view)==E_NOTIMPL &&
        !view,"VIEW_NOT_IMPERSONATED");
    ok &= Check(shell->BindToStorage(item,nullptr,IID_IStream,&view)==E_NOTIMPL &&
        !view,"STORAGE_CLOSED");
    if (parsed) CoTaskMemFree(parsed);
    if (item) CoTaskMemFree(item);
    if (enumeration) enumeration->Release();
    if (persist) persist->Release();
    if (shell) shell->Release();
    folder->Release();
    CoUninitialize();
    if (ok) std::puts("NATIVE_PRIVATE_SHELL_NAMESPACE_REGISTRATIONLESS_CONTRACT_PASS");
    return ok?0:2;
}
