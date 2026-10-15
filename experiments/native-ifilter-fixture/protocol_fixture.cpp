// UNREGISTERED, in-memory Windows Search protocol-accessor prototype.
// All URLs and content are fixed synthetic fixtures. Never touches COM
// registry, SearchHost, Shell, real files, users or installed Search index.
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <searchapi.h>
#include <objidl.h>
#include <atomic>
#include <cstring>
#include <cwchar>
#include <new>
#include <string>

namespace {
constexpr CLSID kProtocolClsid = {
    0x987ad765, 0xd904, 0x4f10, {0xa6, 0x50, 0x91, 0x5d, 0x8f, 0x4c, 0x02, 0x87}};
constexpr wchar_t kUrl[] = L"stfixture-proto-test://root/document";
constexpr wchar_t kName[] = L"document.stfixture";
constexpr wchar_t kMime[] = L"application/x-stfixture-test";
constexpr char kPayload[] =
    u8"STFIXTURE-TEXT-1\nRapor İstanbul Örnek Straße Γειά";
std::atomic<long> g_instances{0};
std::atomic<long> g_locks{0};

HRESULT copy_wide(const wchar_t* source, wchar_t* destination,
                  DWORD capacity, DWORD* required) {
    if (!required) return E_POINTER;
    const auto length = std::wcslen(source) + 1;
    *required = static_cast<DWORD>(length);
    if (!destination || capacity < length) {
        return HRESULT_FROM_WIN32(ERROR_INSUFFICIENT_BUFFER);
    }
    std::memcpy(destination, source, length * sizeof(wchar_t));
    return S_OK;
}

class Accessor final : public IUrlAccessor {
public:
    Accessor() { ++g_instances; }
    ~Accessor() { --g_instances; }
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** result) override {
        if (!result) return E_POINTER;
        *result = nullptr;
        if (iid != IID_IUnknown && iid != __uuidof(IUrlAccessor)) return E_NOINTERFACE;
        *result = static_cast<IUrlAccessor*>(this);
        AddRef();
        return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return ++refs_; }
    ULONG STDMETHODCALLTYPE Release() override {
        ULONG remaining = --refs_;
        if (remaining == 0) delete this;
        return remaining;
    }
    HRESULT STDMETHODCALLTYPE AddRequestParameter(PROPSPEC*, PROPVARIANT*) override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE GetDocFormat(WCHAR* out, DWORD capacity,
                                           DWORD* required) override {
        return copy_wide(kMime, out, capacity, required);
    }
    HRESULT STDMETHODCALLTYPE GetCLSID(CLSID* clsid) override {
        if (!clsid) return E_POINTER;
        *clsid = CLSID_NULL;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE GetHost(WCHAR*, DWORD, DWORD*) override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE IsDirectory() override { return S_FALSE; }
    HRESULT STDMETHODCALLTYPE GetSize(ULONGLONG* bytes) override {
        if (!bytes) return E_POINTER;
        *bytes = sizeof(kPayload) - 1;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE GetLastModified(FILETIME* stamp) override {
        if (!stamp) return E_POINTER;
        // Fixed, nonzero synthetic timestamp. It never reflects user files.
        ULARGE_INTEGER time{};
        time.QuadPart = 132537600000000000ULL;
        stamp->dwLowDateTime = time.LowPart;
        stamp->dwHighDateTime = time.HighPart;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE GetFileName(WCHAR* out, DWORD capacity,
                                          DWORD* required) override {
        return copy_wide(kName, out, capacity, required);
    }
    HRESULT STDMETHODCALLTYPE GetSecurityDescriptor(BYTE*, DWORD, DWORD*) override {
        // Security, container traversal and a production filter binding
        // MUST be implemented and separately reviewed before VM indexing.
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE GetRedirectedURL(WCHAR*, DWORD, DWORD*) override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE GetSecurityProvider(CLSID* clsid) override {
        if (!clsid) return E_POINTER;
        *clsid = CLSID_NULL;
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE BindToStream(IStream** stream) override {
        if (!stream) return E_POINTER;
        *stream = nullptr;
        IStream* memory = nullptr;
        HRESULT hr = CreateStreamOnHGlobal(nullptr, TRUE, &memory);
        if (FAILED(hr)) return hr;
        ULONG written = 0;
        hr = memory->Write(kPayload, static_cast<ULONG>(sizeof(kPayload) - 1), &written);
        LARGE_INTEGER start{};
        if (SUCCEEDED(hr) && written == sizeof(kPayload) - 1) {
            hr = memory->Seek(start, STREAM_SEEK_SET, nullptr);
        } else if (SUCCEEDED(hr)) {
            hr = STG_E_WRITEFAULT;
        }
        if (FAILED(hr)) {
            memory->Release();
            return hr;
        }
        *stream = memory;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE BindToFilter(IFilter** filter) override {
        if (!filter) return E_POINTER;
        *filter = nullptr;
        // Deliberate unsupported operation: this isolated prototype only
        // proves ISearchProtocol -> IUrlAccessor -> IStream, NOT integration
        // into the OS filter host nor native Start/Win+S search results.
        return E_NOTIMPL;
    }
private:
    std::atomic<ULONG> refs_{1};
};

class Protocol final : public ISearchProtocol {
public:
    Protocol() { ++g_instances; }
    ~Protocol() { --g_instances; }
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** result) override {
        if (!result) return E_POINTER;
        *result = nullptr;
        if (iid != IID_IUnknown && iid != __uuidof(ISearchProtocol)) return E_NOINTERFACE;
        *result = static_cast<ISearchProtocol*>(this);
        AddRef();
        return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return ++refs_; }
    ULONG STDMETHODCALLTYPE Release() override {
        ULONG remaining = --refs_;
        if (!remaining) delete this;
        return remaining;
    }
    HRESULT STDMETHODCALLTYPE Init(TIMEOUT_INFO*, IProtocolHandlerSite*,
                                    PROXY_INFO*) override {
        initialized_ = true;
        return S_OK;  // Test factory only; no host site ever registered.
    }
    HRESULT STDMETHODCALLTYPE CreateAccessor(LPCWSTR url, AUTHENTICATION_INFO*,
        INCREMENTAL_ACCESS_INFO*, ITEM_INFO*, IUrlAccessor** result) override {
        if (!result) return E_POINTER;
        *result = nullptr;
        if (!initialized_) return E_UNEXPECTED;
        if (!url || std::wcscmp(url, kUrl) != 0)
            return HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND);
        auto* accessor = new (std::nothrow) Accessor();
        if (!accessor) return E_OUTOFMEMORY;
        *result = accessor;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE CloseAccessor(IUrlAccessor* accessor) override {
        if (!accessor) return E_POINTER;
        accessor->Release();
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE ShutDown() override {
        initialized_ = false;
        return S_OK;
    }
private:
    std::atomic<ULONG> refs_{1};
    bool initialized_ = false;
};

class ProtocolFactory final : public IClassFactory {
public:
    ProtocolFactory() { ++g_instances; }
    ~ProtocolFactory() { --g_instances; }
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** result) override {
        if (!result) return E_POINTER;
        *result = nullptr;
        if (iid != IID_IUnknown && iid != IID_IClassFactory) return E_NOINTERFACE;
        *result = static_cast<IClassFactory*>(this);
        AddRef();
        return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return ++refs_; }
    ULONG STDMETHODCALLTYPE Release() override {
        ULONG remaining = --refs_;
        if (!remaining) delete this;
        return remaining;
    }
    HRESULT STDMETHODCALLTYPE CreateInstance(IUnknown* outer, REFIID iid,
                                              void** result) override {
        if (!result) return E_POINTER;
        *result = nullptr;
        if (outer) return CLASS_E_NOAGGREGATION;
        auto* protocol = new (std::nothrow) Protocol();
        if (!protocol) return E_OUTOFMEMORY;
        HRESULT hr = protocol->QueryInterface(iid, result);
        protocol->Release();
        return hr;
    }
    HRESULT STDMETHODCALLTYPE LockServer(BOOL lock) override {
        if (lock) {
            ++g_locks;
        } else {
            if (!g_locks.load()) return E_UNEXPECTED;
            --g_locks;
        }
        return S_OK;
    }
private:
    std::atomic<ULONG> refs_{1};
};
} // namespace

STDAPI DllGetClassObject(REFCLSID clsid, REFIID iid, void** result) {
    if (!result) return E_POINTER;
    *result = nullptr;
    if (clsid != kProtocolClsid) return CLASS_E_CLASSNOTAVAILABLE;
    auto* factory = new (std::nothrow) ProtocolFactory();
    if (!factory) return E_OUTOFMEMORY;
    HRESULT hr = factory->QueryInterface(iid, result);
    factory->Release();
    return hr;
}
STDAPI DllCanUnloadNow() {
    return (g_instances.load() == 0 && g_locks.load() == 0) ? S_OK : S_FALSE;
}
// Fail closed if a user invokes regsvr32.exe, even by accident.
STDAPI DllRegisterServer() { return E_ACCESSDENIED; }
STDAPI DllUnregisterServer() { return E_ACCESSDENIED; }
