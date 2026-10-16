// Fully in-process protocol-to-stream COM contract test. Never invokes
// CoCreateInstance or uses registry, Windows Search, Start, or shell services.
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <searchapi.h>
#include "fixture_sid_policy.h"
#include <filterr.h>
#include <objidl.h>
#include <iostream>
#include <string>
#include <vector>
#include <cstring>
#include <cwchar>
#include <atomic>

namespace {
constexpr CLSID kProtocolClsid = {
    0x987ad765, 0xd904, 0x4f10, {0xa6, 0x50, 0x91, 0x5d, 0x8f, 0x4c, 0x02, 0x87}};
constexpr wchar_t kUrl[] = L"stfixture-proto-test://root/document";
constexpr CLSID kFilterClsid = {
    0x44a4b197, 0x7237, 0x4958, {0xa1, 0xd0, 0x7c, 0xb2, 0x6e, 0x80, 0x41, 0x65}};
constexpr CLSID kContainerClsid = {
    0x5965eb3b, 0x44aa, 0x405f, {0x9c, 0x60, 0x13, 0x64, 0x07, 0x4d, 0xf2, 0x97}};
using ClassGetter = HRESULT(__stdcall*)(REFCLSID, REFIID, void**);
using CanUnload = HRESULT(__stdcall*)();
using Registration = HRESULT(__stdcall*)();

bool check(bool condition, const char* message) {
    if (!condition) std::cerr << "FAIL " << message << '\n';
    return condition;
}

// The Search protocol host can run as SYSTEM, LocalService or NetworkService
// for URLs without a user SID. Those identities are NOT a valid user ACL.
bool check_acl_identity_policy() {
    bool ok = check(validate_fixture_acl_identity(nullptr) ==
        HRESULT_FROM_WIN32(ERROR_INVALID_SID), "absent ACL principal rejected");
    BYTE malformed[4]{0, 0, 0, 0};
    ok &= check(validate_fixture_acl_identity(malformed) ==
        HRESULT_FROM_WIN32(ERROR_INVALID_SID), "malformed SID rejected");
    for (auto kind : {
        WinLocalSystemSid, WinLocalServiceSid, WinNetworkServiceSid,
        WinServiceSid, WinWorldSid, WinAnonymousSid,
        WinBuiltinAdministratorsSid, WinBuiltinUsersSid,
        WinBuiltinGuestsSid, WinAuthenticatedUserSid
    }) {
        BYTE buffer[SECURITY_MAX_SID_SIZE]{};
        DWORD needed = sizeof(buffer);
        if (!check(CreateWellKnownSid(kind, nullptr, buffer, &needed) != FALSE,
                   "create synthetic well-known principal")) {
            ok = false;
            continue;
        }
        ok &= check(validate_fixture_acl_identity(buffer) == E_ACCESSDENIED,
                    "service, group and public principals fail closed");
    }
    // A service-specific user SID is not the built-in Service logon SID.
    SID_IDENTIFIER_AUTHORITY nt = SECURITY_NT_AUTHORITY;
    BYTE virtual_service[SECURITY_MAX_SID_SIZE]{};
    ok &= check(InitializeSid(virtual_service, &nt, 2) != FALSE, "synthetic service SID structure");
    if (IsValidSid(virtual_service)) {
        *GetSidSubAuthority(virtual_service, 0) = 80;
        *GetSidSubAuthority(virtual_service, 1) = 4242;
        ok &= check(validate_fixture_acl_identity(virtual_service) == E_ACCESSDENIED,
                    "NT SERVICE specific identity rejected");
        *GetSidSubAuthority(virtual_service, 0) = 82;
        ok &= check(validate_fixture_acl_identity(virtual_service) == E_ACCESSDENIED,
                    "application pool service identity rejected");
    }
    HANDLE token = nullptr;
    if (!check(OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token) != FALSE,
               "test process user token accessible")) return false;
    DWORD size = 0;
    GetTokenInformation(token, TokenUser, nullptr, 0, &size);
    bool valid = size >= sizeof(TOKEN_USER) && size <= 65536;
    if (valid) {
        std::vector<BYTE> bytes(size);
        valid = GetTokenInformation(token, TokenUser, bytes.data(), size,
                                    &size) != FALSE;
        if (valid) {
            const auto* token_user = reinterpret_cast<const TOKEN_USER*>(bytes.data());
            valid = validate_fixture_acl_identity(token_user->User.Sid) == S_OK;
        }
    }
    CloseHandle(token);
    ok &= check(valid, "ordinary in-process user SID remains permitted");
    return ok;
}

// The site is an in-process TEST DOUBLE for Windows Search's documented
// IProtocolHandlerSite::GetFilter. It chooses the second unregistered DLL
// from a direct already-opened factory. No Windows Search service involved.
class FixtureSite final : public IProtocolHandlerSite {
public:
    explicit FixtureSite(ClassGetter getter, bool emulate_missing_extension = false)
        : getter_(getter), emulate_missing_extension_(emulate_missing_extension) {}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** out) override {
        if (!out) return E_POINTER;
        *out = nullptr;
        if (iid != IID_IUnknown && iid != __uuidof(IProtocolHandlerSite))
            return E_NOINTERFACE;
        *out = static_cast<IProtocolHandlerSite*>(this);
        AddRef();
        return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return ++refs_; }
    ULONG STDMETHODCALLTYPE Release() override {
        ULONG left = --refs_;
        if (!left) delete this;
        return left;
    }
    HRESULT STDMETHODCALLTYPE GetFilter(CLSID* clsid, LPCWSTR mime,
        LPCWSTR extension, IFilter** out) override {
        if (!out) return E_POINTER;
        *out = nullptr;
        // Both documented single-selector forms: an extension without
        // a dot or a CLSID for the private *document type* (not IFilter).
        // Emulate the real hosted GTHR_E_FILTER_NOT_FOUND on extension
        // lookup to exercise the fallback without registering anything.
        bool document = false;
        bool container = false;
        if (!clsid && !mime && extension) {
            document = std::wcscmp(extension, L"stfixture") == 0;
            container = std::wcscmp(extension, L"stfixture-root") == 0;
            if (!document && !container) return E_INVALIDARG;
            if (emulate_missing_extension_)
                return static_cast<HRESULT>(0x80040D16u);
        } else if (clsid && !mime && !extension) {
            constexpr CLSID docClass = {
                0xa14bb0ef, 0x4769, 0x42ec,
                {0x9b, 0x6a, 0x0b, 0x06, 0xe0, 0xb1, 0x42, 0x01}};
            constexpr CLSID containerClass = {
                0x4a45139f, 0xbb04, 0x4c20,
                {0xa6, 0x9b, 0x8f, 0x20, 0x49, 0xc9, 0x19, 0xf0}};
            document = *clsid == docClass;
            container = *clsid == containerClass;
            if (!document && !container) return E_INVALIDARG;
        } else {
            return E_INVALIDARG;
        }
        const CLSID& resolved = document ? kFilterClsid : kContainerClsid;
        IClassFactory* factory = nullptr;
        HRESULT hr = getter_(resolved, IID_IClassFactory,
                             reinterpret_cast<void**>(&factory));
        if (FAILED(hr)) return hr;
        hr = factory->CreateInstance(nullptr, IID_IFilter,
                                     reinterpret_cast<void**>(out));
        factory->Release();
        return hr;
    }
private:
    std::atomic<ULONG> refs_{1};
    ClassGetter getter_;
    bool emulate_missing_extension_ = false;
};
}
int wmain(int argc, wchar_t** argv) {
    if (argc != 3) return 1;
    HRESULT init = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(init)) return 2;
    HMODULE dll = LoadLibraryExW(argv[1], nullptr, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR);
    if (!dll) { CoUninitialize(); return 3; }
    HMODULE filter_dll = LoadLibraryExW(argv[2], nullptr, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR);
    if (!filter_dll) { FreeLibrary(dll); CoUninitialize(); return 3; }
    auto filter_get = reinterpret_cast<ClassGetter>(
        GetProcAddress(filter_dll, "DllGetClassObject"));
    auto filter_unload = reinterpret_cast<CanUnload>(
        GetProcAddress(filter_dll, "DllCanUnloadNow"));
    auto get = reinterpret_cast<ClassGetter>(GetProcAddress(dll, "DllGetClassObject"));
    auto can = reinterpret_cast<CanUnload>(GetProcAddress(dll, "DllCanUnloadNow"));
    auto reg = reinterpret_cast<Registration>(GetProcAddress(dll, "DllRegisterServer"));
    auto unreg = reinterpret_cast<Registration>(GetProcAddress(dll, "DllUnregisterServer"));
    bool ok = check(get && can && reg && unreg && filter_get && filter_unload,
                    "both COM DLL exports available");
    ok &= check_acl_identity_policy();
    if (ok) {
        ok &= check(can() == S_OK, "empty initial COM state");
        ok &= check(reg() == E_ACCESSDENIED && unreg() == E_ACCESSDENIED,
                    "registration explicitly refused");
        IClassFactory* factory = nullptr;
        CLSID bad{};
        ok &= check(get(bad, IID_IClassFactory,
                        reinterpret_cast<void**>(&factory))
                        == CLASS_E_CLASSNOTAVAILABLE, "unknown CLSID rejected");
        ok &= check(get(kProtocolClsid, IID_IClassFactory,
                        reinterpret_cast<void**>(&factory)) == S_OK && factory,
                    "private protocol CLSID factory");
        if (factory) {
            ok &= check(can() == S_FALSE, "factory keeps DLL live");
            ISearchProtocol* protocol = nullptr;
            ok &= check(factory->CreateInstance(nullptr, __uuidof(ISearchProtocol),
                        reinterpret_cast<void**>(&protocol)) == S_OK && protocol,
                        "ISearchProtocol created");
            if (protocol) {
                IUrlAccessor* accessor = nullptr;
                ok &= check(protocol->CreateAccessor(kUrl, nullptr, nullptr,
                            nullptr, &accessor) == E_UNEXPECTED && accessor == nullptr,
                            "not initialized accessor rejected");
                ok &= check(protocol->Init(nullptr, nullptr, nullptr) == S_OK,
                            "direct factory initialization");
                ok &= check(protocol->CreateAccessor(L"file:///C:/user/private.txt",
                            nullptr, nullptr, nullptr, &accessor)
                            == HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND) && !accessor,
                            "existing user file URL rejected");
                ok &= check(protocol->CreateAccessor(
                    L"stfixture-proto-test://root/unknown", nullptr,
                    nullptr, nullptr, &accessor)
                    == HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND) && !accessor,
                    "out-of-scope fixture URL rejected");
                ok &= check(protocol->CreateAccessor(kUrl, nullptr, nullptr,
                            nullptr, &accessor) == S_OK && accessor,
                            "synthetic fixture accessor created");
                if (accessor) {
                    wchar_t file_name[80]{};
                    DWORD needed = 0;
                    ok &= check(accessor->GetFileName(file_name, 1, &needed) ==
                                HRESULT_FROM_WIN32(ERROR_INSUFFICIENT_BUFFER) &&
                                needed > 1, "file name small buffer rejected");
                    ok &= check(accessor->GetFileName(file_name, 80, &needed) ==
                                S_OK && std::wstring(file_name) == L"document.stfixture",
                                "synthetic file name");
                    wchar_t mime[80]{};
                    ok &= check(accessor->GetDocFormat(mime, 80, &needed) == S_OK &&
                                std::wstring(mime) == L"application/x-stfixture-test",
                                "synthetic format");
                    ok &= check(accessor->IsDirectory() == S_FALSE,
                                "fixture is not a directory");
                    ULONGLONG expected_size = 0;
                    ok &= check(accessor->GetSize(&expected_size) == S_OK
                                && expected_size > 20, "fixture reports size");
                    FILETIME changed{};
                    ok &= check(accessor->GetLastModified(&changed) == S_OK
                                && (changed.dwLowDateTime || changed.dwHighDateTime),
                                "synthetic timestamp");
                    IFilter* unsupported = nullptr;
                    ok &= check(accessor->BindToFilter(&unsupported) == E_NOTIMPL
                                && unsupported == nullptr,
                                "OS filter binding not claimed");
                    // A no-SID URL must NOT return E_NOTIMPL: Windows
                    // Search otherwise allows all users to query the item.
                    DWORD acl_bytes = 0;
                    ok &= check(accessor->GetSecurityDescriptor(nullptr, 0,
                        nullptr) == E_POINTER,
                        "ACL length output is mandatory");
                    ok &= check(accessor->GetSecurityDescriptor(nullptr, 0,
                        &acl_bytes) == HRESULT_FROM_WIN32(ERROR_INSUFFICIENT_BUFFER)
                        && acl_bytes > sizeof(SECURITY_DESCRIPTOR),
                        "no-SID URL has a bounded nonpublic ACL");
                    std::vector<BYTE> security(acl_bytes);
                    DWORD actually_written = 0;
                    ok &= check(accessor->GetSecurityDescriptor(
                        security.data(), static_cast<DWORD>(security.size()),
                        &actually_written) == S_OK &&
                        actually_written == security.size(),
                        "complete self-relative ACL buffer");
                    if (actually_written == security.size() && !security.empty()) {
                        auto* sd = reinterpret_cast<PSECURITY_DESCRIPTOR>(
                            security.data());
                        SECURITY_DESCRIPTOR_CONTROL control = 0;
                        DWORD revision = 0;
                        ok &= check(IsValidSecurityDescriptor(sd) &&
                            GetSecurityDescriptorControl(sd, &control, &revision) &&
                            (control & SE_SELF_RELATIVE),
                            "valid self-relative security descriptor");
                        BOOL present = FALSE;
                        BOOL defaulted = FALSE;
                        PACL acl = nullptr;
                        ok &= check(GetSecurityDescriptorDacl(sd, &present,
                            &acl, &defaulted) && present && acl &&
                            acl->AceCount == 1, "one explicit restricted DACL ACE");
                        if (acl && acl->AceCount == 1) {
                            void* raw_ace = nullptr;
                            ok &= check(GetAce(acl, 0, &raw_ace) && raw_ace,
                                "restricted ACE readable");
                            if (raw_ace) {
                                auto* ace = reinterpret_cast<ACCESS_ALLOWED_ACE*>(raw_ace);
                                PSID sid = &ace->SidStart;
                                ok &= check(ace->Header.AceType == ACCESS_ALLOWED_ACE_TYPE &&
                                    IsValidSid(sid) &&
                                    !IsWellKnownSid(sid, WinWorldSid) &&
                                    (ace->Mask & FILE_GENERIC_READ) == FILE_GENERIC_READ,
                                    "user-only read ACE, never Everyone");
                                PSID owner = nullptr;
                                BOOL owner_defaulted = FALSE;
                                ok &= check(GetSecurityDescriptorOwner(sd, &owner,
                                    &owner_defaulted) && owner && EqualSid(owner, sid),
                                    "DACL principal is the sole descriptor owner");
                                // The one allowed SID must be the caller's
                                // actual token user, not BUILTIN/Users,
                                // Authenticated Users or an unrelated user.
                                HANDLE token = nullptr;
                                if (!OpenProcessToken(GetCurrentProcess(),
                                    TOKEN_QUERY, &token)) {
                                    ok = false;
                                } else {
                                    DWORD needed_token = 0;
                                    GetTokenInformation(token, TokenUser, nullptr,
                                        0, &needed_token);
                                    if (!needed_token || needed_token > 65536) {
                                        ok = false;
                                    } else {
                                        std::vector<BYTE> token_bytes(needed_token);
                                        if (!GetTokenInformation(token, TokenUser,
                                            token_bytes.data(), needed_token,
                                            &needed_token)) {
                                            ok = false;
                                        } else {
                                            const auto* user = reinterpret_cast<
                                                const TOKEN_USER*>(token_bytes.data());
                                            ok &= check(EqualSid(sid, user->User.Sid),
                                                "read permission restricted to process user SID");
                                        }
                                    }
                                    CloseHandle(token);
                                }
                            }
                        }
                    }
                    // An undersized buffer must never be written.
                    BYTE guard[2]{0xa5, 0x5a};
                    DWORD short_needed = 0;
                    ok &= check(accessor->GetSecurityDescriptor(guard, sizeof(guard),
                        &short_needed) ==
                        HRESULT_FROM_WIN32(ERROR_INSUFFICIENT_BUFFER) &&
                        short_needed == acl_bytes &&
                        guard[0] == 0xa5 && guard[1] == 0x5a,
                        "short ACL buffer rejected without partial write");
                    IStream* stream = nullptr;
                    ok &= check(accessor->BindToStream(&stream) == S_OK
                                && stream, "stream binding succeeds");
                    if (stream) {
                        std::vector<char> data;
                        char readbuf[7];
                        for (;;) {
                            ULONG got = 0;
                            HRESULT hr = stream->Read(readbuf, sizeof(readbuf), &got);
                            if (FAILED(hr)) { ok = false; break; }
                            data.insert(data.end(), readbuf, readbuf + got);
                            if (hr == S_FALSE || !got) break;
                        }
                        const std::string full(data.begin(), data.end());
                        const std::string expected =
                            u8"STFIXTURE-TEXT-1\nRapor İstanbul Örnek Straße Γειά";
                        ok &= check(full == expected && data.size() == expected_size,
                                    "byte-for-byte UTF-8 content and length");
                        stream->Release();
                    }
                    // Full synthetic pipeline: protocol URL -> bound IStream
                    // -> second private DLL's real IFilter text chunks.
                    // Neither DLL is registered with Windows Search.
                    IClassFactory* filter_factory = nullptr;
                    ok &= check(filter_get(kFilterClsid, IID_IClassFactory,
                        reinterpret_cast<void**>(&filter_factory)) == S_OK &&
                        filter_factory, "independent IFilter class factory");
                    if (filter_factory) {
                        IFilter* filter = nullptr;
                        ok &= check(filter_factory->CreateInstance(nullptr, IID_IFilter,
                            reinterpret_cast<void**>(&filter)) == S_OK && filter,
                            "second COM DLL filter object");
                        if (filter) {
                            IPersistStream* persist = nullptr;
                            ok &= check(filter->QueryInterface(IID_IPersistStream,
                                reinterpret_cast<void**>(&persist)) == S_OK && persist,
                                "filter stream persistence");
                            if (persist) {
                                IStream* feed = nullptr;
                                ok &= check(accessor->BindToStream(&feed) == S_OK && feed,
                                    "protocol supplies second fresh stream");
                                if (feed) {
                                    ok &= check(persist->Load(feed) == S_OK,
                                        "IFilter loads real protocol bytes");
                                    feed->Release();
                                }
                                ULONG flags = 0;
                                ok &= check(filter->Init(0, 0, nullptr, &flags) == S_OK,
                                    "IFilter initialized after protocol stream");
                                STAT_CHUNK chunk{};
                                ok &= check(filter->GetChunk(&chunk) == S_OK &&
                                    chunk.flags == CHUNK_TEXT, "protocol content text chunk");
                                std::wstring combined;
                                for (int part = 0; part < 256; ++part) {
                                    WCHAR buffer[9]{};
                                    ULONG length = 8;
                                    HRESULT hr = filter->GetText(&length, buffer);
                                    if (hr == FILTER_E_NO_MORE_TEXT) break;
                                    if (FAILED(hr) || length == 0 || length > 8) {
                                        ok = false;
                                        break;
                                    }
                                    combined.append(buffer, length);
                                    if (hr == FILTER_S_LAST_TEXT) break;
                                }
                                ok &= check(combined == L"Rapor İstanbul Örnek Straße Γειά",
                                    "end-to-end protocol stream to IFilter Unicode text");
                                persist->Release();
                            }
                            filter->Release();
                        }
                        filter_factory->Release();
                    }
                    ok &= check(filter_unload() == S_OK,
                        "separate IFilter DLL COM objects released");
                    ok &= check(protocol->CloseAccessor(accessor) == S_OK,
                                "accessor closed and released");
                }

                // The no-host path above must fail closed. Now inject an
                // explicit in-process fake filter host and test the real
                // IUrlAccessor::BindToFilter method, not a manual bypass.
                auto* site = new FixtureSite(filter_get, true);
                ok &= check(protocol->Init(nullptr, site, nullptr) == S_OK,
                    "protocol accepts dedicated test filter site");
                site->Release(); // protocol retains its own reference
                IUrlAccessor* hosted_accessor = nullptr;
                ok &= check(protocol->CreateAccessor(kUrl, nullptr, nullptr,
                    nullptr, &hosted_accessor) == S_OK && hosted_accessor,
                    "accessor inherits host site");
                if (hosted_accessor) {
                    IFilter* hosted_filter = nullptr;
                    ok &= check(hosted_accessor->BindToFilter(&hosted_filter)
                        == S_OK && hosted_filter,
                        "GetFilter test host produces loaded IFilter");
                    if (hosted_filter) {
                        ULONG flags = 0;
                        ok &= check(hosted_filter->Init(0, 0, nullptr, &flags) == S_OK,
                            "host-provided IFilter initialized");
                        STAT_CHUNK chunk{};
                        ok &= check(hosted_filter->GetChunk(&chunk) == S_OK &&
                            chunk.flags == CHUNK_TEXT,
                            "host-provided IFilter returns text chunk");
                        std::wstring extracted;
                        for (int part = 0; part < 256; ++part) {
                            WCHAR buffer[7]{};
                            ULONG length = 6;
                            HRESULT hr = hosted_filter->GetText(&length, buffer);
                            if (hr == FILTER_E_NO_MORE_TEXT) break;
                            if (FAILED(hr) || length == 0 || length > 6) {
                                ok = false;
                                break;
                            }
                            extracted.append(buffer, length);
                            if (hr == FILTER_S_LAST_TEXT) break;
                        }
                        ok &= check(extracted == L"Rapor İstanbul Örnek Straße Γειά",
                            "hosted filter UTF-16 text matches synthetic source");
                        hosted_filter->Release();
                    }
                    ok &= check(protocol->CloseAccessor(hosted_accessor) == S_OK,
                        "test-site accessor closed");
                }

                // Hierarchical crawler path, root with and without slash:
                // the real Windows Search model enumerates child links via
                // CHUNK_VALUE / PKEY_Search_UrlToIndex, not folder filenames.
                for (const wchar_t* root_url : {
                        L"stfixture-proto-test://root",
                        L"stfixture-proto-test://root/"}) {
                    IUrlAccessor* root = nullptr;
                    ok &= check(protocol->CreateAccessor(root_url, nullptr,
                        nullptr, nullptr, &root) == S_OK && root,
                        "synthetic search root recognized (trimmed or slash)");
                    if (!root) continue;
                    ok &= check(root->IsDirectory() == S_OK,
                        "synthetic root IsDirectory");
                    wchar_t root_name[32]{};
                    DWORD root_name_length = 0;
                    ok &= check(root->GetFileName(root_name, 32,
                        &root_name_length) == S_OK &&
                        std::wstring(root_name) == L"root",
                        "root name matches isolated scheme");
                    wchar_t root_mime[80]{};
                    ok &= check(root->GetDocFormat(root_mime, 80,
                        &root_name_length) == S_OK &&
                        std::wstring(root_mime) ==
                        L"application/x-stfixture-container",
                        "root MIME uses container filter");
                    DWORD root_acl_needed = 0;
                    ok &= check(root->GetSecurityDescriptor(nullptr, 0,
                        &root_acl_needed) ==
                        HRESULT_FROM_WIN32(ERROR_INSUFFICIENT_BUFFER) &&
                        root_acl_needed > sizeof(SECURITY_DESCRIPTOR),
                        "root ACL fails closed rather than public access");
                    ULONGLONG root_size = 9;
                    ok &= check(root->GetSize(&root_size) == S_OK && root_size == 0,
                        "root has no file payload");
                    IStream* root_stream = nullptr;
                    ok &= check(root->BindToStream(&root_stream) == E_NOTIMPL &&
                        root_stream == nullptr, "container stream not a file");
                    IFilter* folder = nullptr;
                    ok &= check(root->BindToFilter(&folder) == S_OK && folder,
                        "host provides private container filter");
                    if (folder) {
                        ULONG flags = 1;
                        ok &= check(folder->Init(0, 0, nullptr, &flags) == S_OK &&
                            flags == 0, "container filter initialized");
                        STAT_CHUNK property{};
                        ok &= check(folder->GetChunk(&property) == S_OK &&
                            property.flags == CHUNK_VALUE &&
                            property.attribute.psProperty.ulKind == PRSPEC_PROPID &&
                            property.attribute.psProperty.propid == 2 &&
                            property.attribute.guidPropSet == GUID{
                                0x0b63e343, 0x9ccc, 0x11d0,
                                {0xbc, 0xdb, 0x00, 0x80, 0x5f, 0xcc, 0xce, 0x04}},
                            "container emits PKEY_Search_UrlToIndex property");
                        PROPVARIANT* child = nullptr;
                        ok &= check(folder->GetText(nullptr, nullptr) ==
                            FILTER_E_NO_TEXT, "container is URL metadata, not text");
                        ok &= check(folder->GetValue(&child) == S_OK && child &&
                            child->vt == VT_LPWSTR && child->pwszVal &&
                            std::wstring(child->pwszVal) == kUrl,
                            "one exact synthetic child URL");
                        if (child) {
                            PropVariantClear(child);
                            CoTaskMemFree(child);
                        }
                        child = nullptr;
                        ok &= check(folder->GetValue(&child) == FILTER_E_NO_MORE_VALUES &&
                            child == nullptr, "child URL value cannot duplicate");
                        ok &= check(folder->GetChunk(&property) == FILTER_E_END_OF_CHUNKS,
                            "root contains exactly one child URL chunk");
                        // One more read after reset must return the same one
                        // URL; no implicit crawler recursion or extra items.
                        ok &= check(folder->Init(0, 0, nullptr, &flags) == S_OK &&
                            folder->GetChunk(&property) == S_OK,
                            "container repeat crawl deterministic");
                        child = nullptr;
                        ok &= check(folder->GetValue(&child) == S_OK && child &&
                            child->vt == VT_LPWSTR &&
                            std::wstring(child->pwszVal) == kUrl,
                            "repeat crawl returns same child");
                        if (child) {
                            PropVariantClear(child);
                            CoTaskMemFree(child);
                        }
                        folder->Release();
                    }
                    ok &= check(protocol->CloseAccessor(root) == S_OK,
                        "root accessor released");
                }
                // All other descendants and all real filesystem URLs
                // must remain rejected.
                IUrlAccessor* unsupported_child = nullptr;
                ok &= check(protocol->CreateAccessor(
                    L"stfixture-proto-test://root/document/secret", nullptr,
                    nullptr, nullptr, &unsupported_child) ==
                    HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND) &&
                    !unsupported_child, "unknown nested child rejected");
                ok &= check(filter_unload() == S_OK,
                    "test site retained no filter COM objects");
                ok &= check(protocol->ShutDown() == S_OK, "protocol shuts down");
                IUrlAccessor* after = nullptr;
                ok &= check(protocol->CreateAccessor(kUrl, nullptr, nullptr,
                            nullptr, &after) == E_UNEXPECTED && !after,
                            "shutdown rejects new accessors");
                protocol->Release();
            }
            ok &= check(factory->LockServer(TRUE) == S_OK && can() == S_FALSE,
                        "explicit server lock");
            ok &= check(factory->LockServer(FALSE) == S_OK, "unlock server");
            factory->Release();
        }
        ok &= check(can() == S_OK, "all protocol/accessor objects released");
    }
    FreeLibrary(filter_dll);
    FreeLibrary(dll);
    CoUninitialize();
    if (!ok) return 4;
    std::cout << "NATIVE_PROTOCOL_ROOT_CHILD_IFILTER_ACL_NO_REGISTRATION_PASS\n";
    return 0;
}
