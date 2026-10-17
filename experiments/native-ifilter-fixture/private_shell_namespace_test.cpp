// Registrationless, in-process Shell namespace contract experiment.
// ONE synthetic private item ONLY; never register an Explorer namespace
// extension, never launch explorer.exe, and never read user files.
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <shlobj.h>
#include <shobjidl.h>
#include <propkey.h>
#include <oleauto.h>
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
class PrivateFolder final:public IShellFolder2,public IPersistFolder2 {
public:
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id,void** out) override {
        if (!out) return E_POINTER;
        *out=nullptr;
        if (id==IID_IUnknown || id==IID_IShellFolder || id==IID_IShellFolder2)
            *out=static_cast<IShellFolder2*>(this);
        else if (id==IID_IPersist || id==IID_IPersistFolder ||
                 id==IID_IPersistFolder2)
            *out=static_cast<IPersistFolder2*>(this);
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
    ~PrivateFolder() {
        if (root_) CoTaskMemFree(root_);
    }
    HRESULT STDMETHODCALLTYPE Initialize(PCIDLIST_ABSOLUTE root) override {
        if (!root) return E_INVALIDARG;
        // The caller retains the input absolute PIDL. Own an independent
        // Shell-allocated copy for GetCurFolder and release it on shutdown.
        auto* copy=ILCloneFull(root);
        if (!copy) return E_OUTOFMEMORY;
        if (root_) CoTaskMemFree(root_);
        root_=copy;
        initialized_=true;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE GetCurFolder(PIDLIST_ABSOLUTE* out) override {
        if (!out) return E_POINTER;
        *out=nullptr;
        if (!root_) return S_FALSE;
        *out=ILCloneFull(root_);
        return *out ? S_OK : E_OUTOFMEMORY;
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
    HRESULT STDMETHODCALLTYPE CreateViewObject(HWND,REFIID requested,void** out) override {
        if (!out) return E_POINTER;
        *out=nullptr;
        if (!initialized_) return E_UNEXPECTED;
        if (requested!=IID_IShellView) return E_NOINTERFACE;
        // Microsoft's supported default DefView factory is given only
        // this in-memory IShellFolder. This creates an IShellView COM
        // object, NOT an Explorer window or registered namespace entry.
        SFV_CREATE config{};
        config.cbSize=sizeof(config);
        config.pshf=static_cast<IShellFolder*>(static_cast<IShellFolder2*>(this));
        IShellView* view=nullptr;
        const HRESULT hr=SHCreateShellFolderView(&config,&view);
        if (FAILED(hr)) {
            if (view) view->Release();
            return hr;
        }
        if (!view) return E_UNEXPECTED;
        *out=view;
        return S_OK;
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
    // Two fixture-only Explorer detail columns: display name and its
    // immutable parsing URL. No filesystem attributes or user items.
    HRESULT STDMETHODCALLTYPE GetDefaultSearchGUID(GUID* out) override {
        if (!out) return E_POINTER;
        *out=GUID{};
        return E_NOTIMPL; // No Search UI extension or custom SearchHost entrypoint.
    }
    HRESULT STDMETHODCALLTYPE EnumSearches(IEnumExtraSearch** out) override {
        if (!out) return E_POINTER;
        *out=nullptr;
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE GetDefaultColumn(DWORD reserved,ULONG* sort,ULONG* display) override {
        if (reserved || !sort || !display) return E_INVALIDARG;
        *sort=0; *display=0;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE GetDefaultColumnState(UINT column,SHCOLSTATEF* flags) override {
        if (!flags) return E_POINTER;
        *flags=static_cast<SHCOLSTATEF>(0);
        if (column>1) return E_INVALIDARG;
        *flags=static_cast<SHCOLSTATEF>(SHCOLSTATE_TYPE_STR |
            (column==0 ? SHCOLSTATE_ONBYDEFAULT : 0));
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE MapColumnToSCID(UINT column,SHCOLUMNID* key) override {
        if (!key) return E_POINTER;
        *key=SHCOLUMNID{};
        if (column>1) return E_INVALIDARG;
        *key=column==0 ? PKEY_ItemNameDisplay : PKEY_ParsingPath;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE GetDetailsEx(PCUITEMID_CHILD item,const SHCOLUMNID* key,VARIANT* out) override {
        if (!key || !out) return E_POINTER;
        VariantInit(out);
        if (!IsSyntheticPidl(item)) return E_INVALIDARG;
        PCWSTR value=nullptr;
        if (*key==PKEY_ItemNameDisplay) value=kItemName;
        else if (*key==PKEY_ParsingPath) value=kItemUrl;
        else return E_INVALIDARG;
        out->vt=VT_BSTR;
        out->bstrVal=SysAllocString(value);
        if (!out->bstrVal) {
            VariantInit(out);
            return E_OUTOFMEMORY;
        }
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE GetDetailsOf(PCUITEMID_CHILD item,UINT column,SHELLDETAILS* out) override {
        if (!out) return E_POINTER;
        *out=SHELLDETAILS{};
        if (column>1) return E_INVALIDARG;
        if (item && !IsSyntheticPidl(item)) return E_INVALIDARG;
        out->fmt=LVCFMT_LEFT;
        out->cxChar=column==0 ? 28 : 48;
        return ToStrret(item ? (column==0 ? kItemName : kItemUrl) :
            (column==0 ? L"Name" : L"Parsing URL"),&out->str);
    }
private:
    std::atomic<ULONG> refs_{1};
    bool initialized_=false;
    PIDLIST_ABSOLUTE root_=nullptr;
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
    IShellFolder2* details=nullptr;
    IPersistFolder* persist=nullptr;
    IPersistFolder2* persist2=nullptr;
    ok &= Check(folder->QueryInterface(IID_IShellFolder,
        reinterpret_cast<void**>(&shell))==S_OK && shell,"QUERY_FOLDER");
    ok &= Check(folder->QueryInterface(IID_IPersistFolder,
        reinterpret_cast<void**>(&persist))==S_OK && persist,"QUERY_PERSIST");
    ok &= Check(folder->QueryInterface(IID_IShellFolder2,
        reinterpret_cast<void**>(&details))==S_OK && details,"QUERY_DETAILS");
    ok &= Check(folder->QueryInterface(IID_IPersistFolder2,
        reinterpret_cast<void**>(&persist2))==S_OK && persist2,"QUERY_PERSIST2");
    PIDLIST_ABSOLUTE beforeInitialize=reinterpret_cast<PIDLIST_ABSOLUTE>(0x1);
    ok &= Check(persist2 && persist2->GetCurFolder(&beforeInitialize)==S_FALSE &&
        !beforeInitialize,"PERSIST2_NOT_INITIALIZED");
    void* prematureView=reinterpret_cast<void*>(0x1);
    ok &= Check(shell && shell->CreateViewObject(nullptr,IID_IShellView,
        &prematureView)==E_UNEXPECTED && !prematureView,
        "UNINITIALIZED_SHELL_VIEW_FAILS_CLOSED");
    CLSID id{};
    ok &= Check(persist && persist->GetClassID(&id)==S_OK &&
        id==kFixtureShellClsid,"CLASS_ID");
    const USHORT root[]={4,0x1717,0};
    ok &= Check(persist && persist->Initialize(
        reinterpret_cast<PCIDLIST_ABSOLUTE>(root))==S_OK,"ROOT_INIT");
    PIDLIST_ABSOLUTE cloneRoot=nullptr;
    ok &= Check(persist2->GetCurFolder(&cloneRoot)==S_OK &&
        cloneRoot && reinterpret_cast<const void*>(cloneRoot)!=root &&
        ILGetSize(cloneRoot)==sizeof(root) &&
        std::memcmp(cloneRoot,root,sizeof(root))==0,"PERSIST2_ROOT_CLONE");
    if (cloneRoot) {
        // The first returned PIDL belongs exclusively to the caller.
        reinterpret_cast<BYTE*>(cloneRoot)[2]=0;
        CoTaskMemFree(cloneRoot);
    }
    PIDLIST_ABSOLUTE freshRoot=nullptr;
    ok &= Check(persist2->GetCurFolder(&freshRoot)==S_OK &&
        freshRoot && std::memcmp(freshRoot,root,sizeof(root))==0,
        "PERSIST2_ROOT_CLONE_INDEPENDENT");
    if (freshRoot) CoTaskMemFree(freshRoot);
    ULONG sortColumn=99,displayColumn=99;
    ok &= Check(details && details->GetDefaultColumn(0,&sortColumn,&displayColumn)==S_OK &&
        sortColumn==0 && displayColumn==0,"DEFAULT_COLUMN");
    SHCOLUMNID nameKey{},urlKey{},invalidKey{};
    ok &= Check(details->MapColumnToSCID(0,&nameKey)==S_OK &&
        nameKey==PKEY_ItemNameDisplay,"NAME_COLUMN_KEY");
    ok &= Check(details->MapColumnToSCID(1,&urlKey)==S_OK &&
        urlKey==PKEY_ParsingPath,"URL_COLUMN_KEY");
    ok &= Check(details->MapColumnToSCID(2,&invalidKey)==E_INVALIDARG &&
        invalidKey==SHCOLUMNID{},"NO_USER_COLUMN");
    ok &= Check(details->GetDefaultColumn(1,&sortColumn,&displayColumn)==E_INVALIDARG,
        "DEFAULT_COLUMN_REJECT_RESERVED");
    SHCOLSTATEF nameState=static_cast<SHCOLSTATEF>(0),urlState=static_cast<SHCOLSTATEF>(0);
    ok &= Check(details->GetDefaultColumnState(0,&nameState)==S_OK &&
        (nameState & SHCOLSTATE_TYPE_STR) && (nameState & SHCOLSTATE_ONBYDEFAULT),
        "NAME_COLUMN_STATE");
    ok &= Check(details->GetDefaultColumnState(1,&urlState)==S_OK &&
        (urlState & SHCOLSTATE_TYPE_STR) && !(urlState & SHCOLSTATE_ONBYDEFAULT),
        "URL_COLUMN_STATE");
    ok &= Check(details->GetDefaultColumnState(2,&urlState)==E_INVALIDARG &&
        urlState==static_cast<SHCOLSTATEF>(0),"NO_UNKNOWN_COLUMN_STATE");
    IEnumExtraSearch* noSearch=reinterpret_cast<IEnumExtraSearch*>(0x1);
    ok &= Check(details->EnumSearches(&noSearch)==E_NOTIMPL && !noSearch,
        "NO_SEARCH_UI_OVERRIDE");
    GUID noSearchId{};
    ok &= Check(details->GetDefaultSearchGUID(&noSearchId)==E_NOTIMPL,
        "NO_CUSTOM_SEARCH_GUID");
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
    VARIANT detailValue{};
    ok &= Check(details->GetDetailsEx(item,&nameKey,&detailValue)==S_OK &&
        detailValue.vt==VT_BSTR &&
        std::wcscmp(detailValue.bstrVal,kItemName)==0,"ITEM_NAME_DETAIL");
    VariantClear(&detailValue);
    ok &= Check(details->GetDetailsEx(item,&urlKey,&detailValue)==S_OK &&
        detailValue.vt==VT_BSTR &&
        std::wcscmp(detailValue.bstrVal,kItemUrl)==0,"PARSING_URL_DETAIL");
    VariantClear(&detailValue);
    ok &= Check(details->GetDetailsEx(item,&PKEY_ItemTypeText,&detailValue)==E_INVALIDARG &&
        detailValue.vt==VT_EMPTY,"NO_UNTRUSTED_PROPERTY");
    const USHORT foreignPidl[]={4,0x7171,0};
    ok &= Check(details->GetDetailsEx(
        reinterpret_cast<PCUITEMID_CHILD>(foreignPidl),&urlKey,&detailValue)==E_INVALIDARG &&
        detailValue.vt==VT_EMPTY,"NO_UNTRUSTED_ITEM_PROPERTIES");
    SHELLDETAILS rejected{};
    ok &= Check(details->GetDetailsOf(
        reinterpret_cast<PCUITEMID_CHILD>(foreignPidl),0,&rejected)==E_INVALIDARG,
        "NO_UNTRUSTED_ITEM_COLUMN");
    ok &= Check(details->GetDetailsOf(item,2,&rejected)==E_INVALIDARG,
        "NO_UNKNOWN_ITEM_COLUMN");
    SHELLDETAILS header{};
    ok &= Check(details->GetDetailsOf(nullptr,0,&header)==S_OK &&
        header.str.uType==STRRET_WSTR &&
        std::wcscmp(header.str.pOleStr,L"Name")==0,"NAME_HEADER");
    if (header.str.uType==STRRET_WSTR) CoTaskMemFree(header.str.pOleStr);
    SHELLDETAILS itemDetail{};
    ok &= Check(details->GetDetailsOf(item,1,&itemDetail)==S_OK &&
        itemDetail.str.uType==STRRET_WSTR &&
        std::wcscmp(itemDetail.str.pOleStr,kItemUrl)==0,"DETAIL_COLUMN_URL");
    if (itemDetail.str.uType==STRRET_WSTR) CoTaskMemFree(itemDetail.str.pOleStr);
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
    ok &= Check(shell->CreateViewObject(nullptr,IID_IUnknown,&view)==E_NOINTERFACE &&
        !view,"UNSUPPORTED_VIEW_IID_CLOSED");
    ok &= Check(shell->CreateViewObject(nullptr,IID_IShellView,nullptr)==E_POINTER,
        "SHELL_VIEW_REQUIRES_RESULT_POINTER");
    IShellView* firstView=nullptr;
    ok &= Check(shell->CreateViewObject(nullptr,IID_IShellView,
        reinterpret_cast<void**>(&firstView))==S_OK && firstView,
        "DEFAULT_SHELL_VIEW_FACTORY");
    IShellView* secondView=nullptr;
    ok &= Check(shell->CreateViewObject(nullptr,IID_IShellView,
        reinterpret_cast<void**>(&secondView))==S_OK && secondView &&
        firstView!=secondView,"INDEPENDENT_SHELL_VIEW_INSTANCES");
    if (secondView) secondView->Release();
    if (firstView) firstView->Release();
    ok &= Check(shell->BindToStorage(item,nullptr,IID_IStream,&view)==E_NOTIMPL &&
        !view,"STORAGE_CLOSED");
    if (parsed) CoTaskMemFree(parsed);
    if (item) CoTaskMemFree(item);
    if (enumeration) enumeration->Release();
    if (persist) persist->Release();
    if (persist2) persist2->Release();
    if (details) details->Release();
    if (shell) shell->Release();
    folder->Release();
    CoUninitialize();
    if (ok) std::puts("NATIVE_PRIVATE_SHELL_NAMESPACE_REGISTRATIONLESS_CONTRACT_PASS");
    return ok?0:2;
}
