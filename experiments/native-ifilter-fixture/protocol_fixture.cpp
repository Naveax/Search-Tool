// UNREGISTERED, in-memory Windows Search protocol-accessor prototype.
// All URLs and content are fixed synthetic fixtures. Never touches COM
// registry, SearchHost, Shell, real files, users or installed Search index.
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <searchapi.h>
#include "fixture_sid_policy.h"
#include "fixture_etw.h"
#include <objidl.h>
#include <atomic>
#include <cstring>
#include <cwchar>
#include <sddl.h>
#include <new>
#include <string>
#include <vector>

namespace {
constexpr CLSID kProtocolClsid = {
    0x987ad765, 0xd904, 0x4f10, {0xa6, 0x50, 0x91, 0x5d, 0x8f, 0x4c, 0x02, 0x87}};
constexpr wchar_t kUrl[] = L"stfixture-proto-test://root/document";
constexpr wchar_t kRootUrl[] = L"stfixture-proto-test://root";
constexpr wchar_t kRootMime[] = L"application/x-stfixture-container";
constexpr wchar_t kRootFileName[] = L"root";
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
// Windows Search may crawl SID-less synthetic URLs from a SYSTEM/service
// process. Its token is NOT the document owner. In the disposable CI guest
// only, the synthetic test document owner is supplied by a protected,
// temporary HKLM class-registration value written by the test admin.
// Real stores MUST obtain the owner ACL from authoritative source metadata.
struct DisposableFixtureSid {
    PSID value = nullptr;
    ~DisposableFixtureSid() { if (value) LocalFree(value); }
};
HRESULT read_disposable_fixture_owner(DisposableFixtureSid& owner) {
    wchar_t text[192]{};
    DWORD bytes = sizeof(text);
    const LSTATUS status = RegGetValueW(
        HKEY_LOCAL_MACHINE,
        L"SOFTWARE\\Classes\\CLSID\\{987AD765-D904-4F10-A650-915D8F4C0287}",
        L"FixtureOwnerSid", RRF_RT_REG_SZ, nullptr, text, &bytes);
    if (status != ERROR_SUCCESS || bytes <= sizeof(wchar_t) ||
        bytes > sizeof(text)) return E_ACCESSDENIED;
    if (!ConvertStringSidToSidW(text, &owner.value)) return E_ACCESSDENIED;
    return validate_fixture_acl_identity(owner.value);
}

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
    // The real no-SID URL may be hosted by SYSTEM/LocalService. These
    // identities are NOT the end user and must not become the ACL owner.
    // Fail closed until genuine per-user identity/impersonation is proven.
    HRESULT identity = validate_fixture_acl_identity(user_sid);
    DisposableFixtureSid authoritativeOwner;
    if (identity == E_ACCESSDENIED) {
        // Fail closed unless a trusted admin explicitly configured the
        // synthetic fixture owner for this one isolated private CLSID.
        identity = read_disposable_fixture_owner(authoritativeOwner);
        if (FAILED(identity)) return identity;
        user_sid = authoritativeOwner.value;
        private_fixture_trace(113); // Trusted test owner (NOT service token)
    } else if (FAILED(identity)) {
        return identity;
    }
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


// A per-user URL must be bound to the CURRENT effective Windows identity.
// Do not trust a caller-supplied SID, or substitute a Search service token.
bool matches_effective_user_sid(const std::wstring& sid_string) {
    if (sid_string.size() < 9 || sid_string.size() > 184 ||
        sid_string.compare(0, 4, L"S-1-") != 0) return false;
    PSID candidate = nullptr;
    if (!ConvertStringSidToSidW(sid_string.c_str(), &candidate)) return false;
    bool allowed = false;
    if (SUCCEEDED(validate_fixture_acl_identity(candidate))) {
        HANDLE token = nullptr;
        // Prefer actual impersonation identity; do not fall back to a
        // service process token when an impersonated thread cannot be read.
        if (!OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, TRUE, &token)) {
            if (GetLastError() == ERROR_NO_TOKEN) {
                OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token);
            }
        }
        if (token) {
            DWORD needed = 0;
            GetTokenInformation(token, TokenUser, nullptr, 0, &needed);
            if (needed >= sizeof(TOKEN_USER) && needed <= 65536) {
                std::vector<BYTE> buffer(needed);
                if (GetTokenInformation(token, TokenUser, buffer.data(),
                                        needed, &needed)) {
                    const auto* user = reinterpret_cast<const TOKEN_USER*>(buffer.data());
                    allowed = IsValidSid(user->User.Sid) &&
                        EqualSid(user->User.Sid, candidate);
                }
            }
            CloseHandle(token);
        }
    }
    LocalFree(candidate);
    return allowed;
}

bool parse_current_user_fixture_url(LPCWSTR url, bool& directory) {
    directory = false;
    if (!url) return false;
    constexpr wchar_t prefix[] = L"stfixture-proto-test://{";
    const std::wstring input(url);
    if (input.size() > 260 || input.compare(0, std::wcslen(prefix), prefix) != 0)
        return false;
    const auto close = input.find(L'}', std::wcslen(prefix));
    if (close == std::wstring::npos) return false;
    const std::wstring path = input.substr(close + 1);
    if (path == L"/root" || path == L"/root/") {
        directory = true;
    } else if (path != L"/root/document") {
        return false;
    }
    return matches_effective_user_sid(
        input.substr(std::wcslen(prefix), close - std::wcslen(prefix)));
}

class Accessor final : public IUrlAccessor {
public:
    Accessor(IProtocolHandlerSite* site, bool directory, bool user_scoped)
        : site_(site), directory_(directory), user_scoped_(user_scoped) {
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
        return copy_wide(directory_ ? kRootMime : kMime, out, capacity, required);
    }
    HRESULT STDMETHODCALLTYPE GetCLSID(CLSID* clsid) override {
        if (!clsid) return E_POINTER;
        *clsid = CLSID_NULL;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE GetHost(WCHAR*, DWORD, DWORD*) override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE IsDirectory() override {
        return directory_ ? S_OK : S_FALSE;
    }
    HRESULT STDMETHODCALLTYPE GetSize(ULONGLONG* bytes) override {
        if (!bytes) return E_POINTER;
        *bytes = directory_ ? 0 : sizeof(kPayload) - 1;
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
        return copy_wide(directory_ ? kRootFileName : kName, out, capacity, required);
    }
    HRESULT STDMETHODCALLTYPE GetSecurityDescriptor(BYTE* destination,
        DWORD capacity, DWORD* written) override {
        private_fixture_trace(103); // GetSecurityDescriptor entry
        if (!written) return E_POINTER;
        *written = 0;
        // Microsoft requires E_NOTIMPL for a URL with a user SID.
        // This branch is reachable ONLY after validating the current
        // impersonated/process TokenUser against the URL's private SID.
        if (user_scoped_) {
            private_fixture_trace(110); // Scoped user ACL: E_NOTIMPL
            return E_NOTIMPL;
        }
        std::vector<BYTE> descriptor;
        HRESULT hr = build_private_test_descriptor(descriptor);
        if (FAILED(hr)) {
            private_fixture_trace(107); // Private system/service ACL refused
            return hr;
        }
        *written = static_cast<DWORD>(descriptor.size());
        if (!destination || capacity < descriptor.size()) {
            private_fixture_trace(109); // Expected sizing probe
            return HRESULT_FROM_WIN32(ERROR_INSUFFICIENT_BUFFER);
        }
        std::memcpy(destination, descriptor.data(), descriptor.size());
        private_fixture_trace(108); // Restricted ACL returned successfully
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
        private_fixture_trace(105); // BindToStream entry
        if (!stream) return E_POINTER;
        *stream = nullptr;
        if (directory_) return E_NOTIMPL;
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
        private_fixture_trace(104); // BindToFilter entry
        if (!filter) return E_POINTER;
        *filter = nullptr;
        // The Search host must provide the handler. Never instantiate a
        // registered CLSID, load another DLL path, or bypass the host's
        // isolated filter selection/security boundary.
        if (!site_) return E_NOTIMPL;
        // IProtocolHandlerSite::GetFilter expects an extension WITHOUT
        // the leading dot. Supplying MIME, CLSID, and extension together
        // prioritizes MIME lookup and can prevent resolution of our custom
        // persistent handler. Select by exactly ONE registered private
        // extension and let the isolated host create the filter.
        IFilter* candidate = nullptr;
        HRESULT hr = site_->GetFilter(nullptr, nullptr,
                                     directory_ ? L"stfixture-root" : L"stfixture",
                                     &candidate);
        private_fixture_trace(FAILED(hr) ? 120 : 121);
        if (FAILED(hr)) private_fixture_trace_hresult(hr);
        if (FAILED(hr)) {
            // Classify HRESULT in a fixed, no-payload ETW bucket. Never
            // trace URL, extension, token or arbitrary exception messages.
            unsigned short category = 132; // Other HRESULT
            if (hr == static_cast<HRESULT>(0x80040D16u)) category = 136;
            else if (hr == REGDB_E_CLASSNOTREG) category = 124;
            else if (hr == E_ACCESSDENIED) category = 125;
            else if (hr == E_INVALIDARG) category = 126;
            else if (hr == E_NOTIMPL) category = 127;
            else if (hr == HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND)) category = 128;
            else if (hr == HRESULT_FROM_WIN32(ERROR_NOT_FOUND)) category = 129;
            else if (hr == E_FAIL) category = 130;
            else if (hr == HRESULT_FROM_WIN32(ERROR_MOD_NOT_FOUND)) category = 131;
            private_fixture_trace(category);
        }
        if (hr == static_cast<HRESULT>(0x80040D16u)) {
            // GTHR_E_FILTER_NOT_FOUND: a newly registered private
            // extension can be absent from the host's extension table.
            // Request the exact private *document class*, which has its
            // own PersistentHandler association. Never instantiate the
            // IFilter in SearchProtocolHost or bypass host isolation.
            if (candidate) {
                candidate->Release();
                candidate = nullptr;
            }
            CLSID object_type = directory_
                ? CLSID{0x4a45139f, 0xbb04, 0x4c20,
                        {0xa6, 0x9b, 0x8f, 0x20, 0x49, 0xc9, 0x19, 0xf0}}
                : CLSID{0xa14bb0ef, 0x4769, 0x42ec,
                        {0x9b, 0x6a, 0x0b, 0x06, 0xe0, 0xb1, 0x42, 0x01}};
            private_fixture_trace(133); // Known missing-extension fallback
            hr = site_->GetFilter(&object_type,
                                  nullptr, nullptr, &candidate);
            private_fixture_trace(FAILED(hr) ? 134 : 135);
            if (FAILED(hr)) private_fixture_trace_hresult(hr);
        }
        if (hr == static_cast<HRESULT>(0x80040D16u)) {
            // DIAGNOSTIC ONLY: test if the same isolated host can locate
            // Microsoft's built-in plain-text filter. This never changes
            // the txt registration and never hands the built-in filter
            // to the fixture indexer; the original private error remains.
            private_fixture_trace(137);
            IFilter* known = nullptr;
            const HRESULT known_hr = site_->GetFilter(
                nullptr, nullptr, L"txt", &known);
            private_fixture_trace(SUCCEEDED(known_hr) && known ? 138 : 139);
            if (FAILED(known_hr)) private_fixture_trace_hresult(known_hr);
            if (!directory_ && SUCCEEDED(known_hr) && known) {
                // The ONLY payload in this entire fixture is fixed UTF-8
                // plain text. The built-in Windows txt IFilter is therefore
                // a valid host-selected processor for the document itself.
                // Containers still require our PRIVATE URL-enumeration
                // IFilter and must not pretend to be a text document.
                candidate = known;
                hr = known_hr;
                private_fixture_trace(140);
            } else if (known) {
                known->Release();
            }
        }
        if (FAILED(hr)) {
            if (candidate) candidate->Release();
            return hr;
        }
        if (!candidate) return E_UNEXPECTED;
        // Directory filter produces only property-value child URL chunks;
        // it has no content stream and does not implement persistence.
        if (directory_) {
            *filter = candidate;
            return S_OK;
        }
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
    bool directory_ = false;
    bool user_scoped_ = false;
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
        private_fixture_trace(101); // ISearchProtocol::Init entry
        if (site) site->AddRef();
        if (site_) site_->Release();
        site_ = site;
        initialized_ = true;
        return S_OK; // No actual Windows Search host is registered.
    }
    HRESULT STDMETHODCALLTYPE CreateAccessor(LPCWSTR url, AUTHENTICATION_INFO*,
        INCREMENTAL_ACCESS_INFO*, ITEM_INFO*, IUrlAccessor** result) override {
        private_fixture_trace(102); // ISearchProtocol::CreateAccessor entry
        if (!result) return E_POINTER;
        *result = nullptr;
        if (!initialized_) return E_UNEXPECTED;
        // Indexers may trim the trailing slash on root URLs. Both spellings
        // map to the exact same synthetic container, never the local disk.
        const bool is_root = url && (std::wcscmp(url, kRootUrl) == 0 ||
                           std::wcscmp(url, L"stfixture-proto-test://root/") == 0);
        const bool is_document = url && std::wcscmp(url, kUrl) == 0;
        bool scoped_root = false;
        const bool user_scoped = (!is_root && !is_document) &&
            parse_current_user_fixture_url(url, scoped_root);
        if (!is_root && !is_document && !user_scoped) {
            private_fixture_trace(111); // URL rejected by private fixture parser
            return HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND);
        }
        private_fixture_trace(112); // Fixed private URL accepted
        auto* accessor = new (std::nothrow)
            Accessor(site_, is_root || scoped_root, user_scoped);
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
    private_fixture_trace(100); // Private protocol DllGetClassObject
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
