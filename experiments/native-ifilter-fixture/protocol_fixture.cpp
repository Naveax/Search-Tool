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
#include <vector>

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


/*
 * The test URL has NO user SID component. Per Microsoft's IUrlAccessor
 * contract, returning E_NOTIMPL for its security descriptor would make
 * an installed item query-visible to everyone. For this unregistered
 * prototype, produce a valid self-relative DACL granting read access
 * only to the process token's user. The token/SID is never logged or
 * persisted, and this intentionally does NOT solve impersonation or
 * production SecurityProtocolHost service-account mapping.
 */
HRESULT build_private_test_descriptor(std::vector<BYTE>& output) {
    HANDLE token = nullptr;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token)) {
        return HRESULT_FROM_WIN32(GetLastError());
    }
    DWORD required = 0;
    GetTokenInformation(token, TokenUser, nullptr, 0, &required);
    if (required == 0 || required > 65536) {
        CloseHandle(token);
        return HRESULT_FROM_WIN32(ERROR_INVALID_DATA);
    }
    std::vector<BYTE> token_bytes(required);
    if (!GetTokenInformation(token, TokenUser, token_bytes.data(),
                             required, &required)) {
        DWORD error = GetLastError();
        CloseHandle(token);
        return HRESULT_FROM_WIN32(error);
    }
    CloseHandle(token);

    const auto* token_user =
        reinterpret_cast<const TOKEN_USER*>(token_bytes.data());
    PSID user_sid = token_user->User.Sid;
    if (!IsValidSid(user_sid)) return HRESULT_FROM_WIN32(ERROR_INVALID_SID);
    const DWORD sid_size = GetLengthSid(user_sid);
    const DWORD acl_size = sizeof(ACL) + sizeof(ACCESS_ALLOWED_ACE) +
                           sid_size - sizeof(DWORD);
    std::vector<BYTE> acl_bytes(acl_size);
    PACL acl = reinterpret_cast<PACL>(acl_bytes.data());
    if (!InitializeAcl(acl, acl_size, ACL_REVISION) ||
        !AddAccessAllowedAce(acl, ACL_REVISION, FILE_GENERIC_READ, user_sid)) {
        return HRESULT_FROM_WIN32(GetLastError());
    }
    SECURITY_DESCRIPTOR descriptor{};
    if (!InitializeSecurityDescriptor(&descriptor, SECURITY_DESCRIPTOR_REVISION) ||
        !SetSecurityDescriptorOwner(&descriptor, user_sid, FALSE) ||
        !SetSecurityDescriptorDacl(&descriptor, TRUE, acl, FALSE)) {
        return HRESULT_FROM_WIN32(GetLastError());
    }
    DWORD relative_size = 0;
    MakeSelfRelativeSD(&descriptor, nullptr, &relative_size);
    if (relative_size == 0 || relative_size > 65536) {
        return HRESULT_FROM_WIN32(ERROR_INVALID_DATA);
    }
    output.resize(relative_size);
    if (!MakeSelfRelativeSD(&descriptor, output.data(), &relative_size)) {
        return HRESULT_FROM_WIN32(GetLastError());
    }
    output.resize(relative_size);
    return S_OK;
}

class Accessor final : public IUrlAccessor {
public:
    explicit Accessor(IProtocolHandlerSite* site) : site_(site) {
        if (site_) site_->AddRef();
        ++g_instances;
    }
    ~Accessor() {
        if (site_) site_->Release();
        --g_instances;
    }
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
    HRESULT STDMETHODCALLTYPE GetSecurityDescriptor(BYTE* destination,
        DWORD capacity, DWORD* written) override {
        if (!written) return E_POINTER;
        *written = 0;
        std::vector<BYTE> descriptor;
        HRESULT hr = build_private_test_descriptor(descriptor);
        if (FAILED(hr)) return hr;
        *written = static_cast<DWORD>(descriptor.size());
        if (!destination || capacity < descriptor.size()) {
            return HRESULT_FROM_WIN32(ERROR_INSUFFICIENT_BUFFER);
        }
        std::memcpy(destination, descriptor.data(), descriptor.size());
        return S_OK;
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
        // The Search host must provide the handler. Never instantiate a
        // registered CLSID, load another DLL path, or bypass the host's
        // isolated filter selection/security boundary.
        if (!site_) return E_NOTIMPL;
        CLSID clsid = {
            0x44a4b197, 0x7237, 0x4958,
            {0xa1, 0xd0, 0x7c, 0xb2, 0x6e, 0x80, 0x41, 0x65}
        };
        IFilter* candidate = nullptr;
        HRESULT hr = site_->GetFilter(&clsid, kMime, L".stfixture", &candidate);
        if (FAILED(hr)) {
            if (candidate) candidate->Release();
            return hr;
        }
        if (!candidate) return E_UNEXPECTED;
        IPersistStream* persist = nullptr;
        hr = candidate->QueryInterface(IID_IPersistStream,
                                       reinterpret_cast<void**>(&persist));
        if (SUCCEEDED(hr) && persist) {
            IStream* content = nullptr;
            hr = BindToStream(&content);
            if (SUCCEEDED(hr) && content) {
                hr = persist->Load(content);
                content->Release();
            } else if (SUCCEEDED(hr)) {
                hr = E_UNEXPECTED;
            }
            persist->Release();
        } else if (SUCCEEDED(hr)) {
            hr = E_NOINTERFACE;
        }
        if (FAILED(hr)) {
            candidate->Release();
            return hr;
        }
        *filter = candidate;
        return S_OK;
    }
private:
    std::atomic<ULONG> refs_{1};
    IProtocolHandlerSite* site_ = nullptr;
};

class Protocol final : public ISearchProtocol {
public:
    Protocol() { ++g_instances; }
    ~Protocol() {
        if (site_) site_->Release();
        --g_instances;
    }
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
    HRESULT STDMETHODCALLTYPE Init(TIMEOUT_INFO*, IProtocolHandlerSite* site,
                                    PROXY_INFO*) override {
        if (site) site->AddRef();
        if (site_) site_->Release();
        site_ = site;
        initialized_ = true;
        return S_OK; // No actual Windows Search host is registered.
    }
    HRESULT STDMETHODCALLTYPE CreateAccessor(LPCWSTR url, AUTHENTICATION_INFO*,
        INCREMENTAL_ACCESS_INFO*, ITEM_INFO*, IUrlAccessor** result) override {
        if (!result) return E_POINTER;
        *result = nullptr;
        if (!initialized_) return E_UNEXPECTED;
        if (!url || std::wcscmp(url, kUrl) != 0)
            return HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND);
        auto* accessor = new (std::nothrow) Accessor(site_);
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
        if (site_) {
            site_->Release();
            site_ = nullptr;
        }
        return S_OK;
    }
private:
    std::atomic<ULONG> refs_{1};
    IProtocolHandlerSite* site_ = nullptr;
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
