// OS COM runtime activation of a private fixture CLSID, ephemeral guest only.
// The normal protocol_test.cpp continues to exercise unregistered COM. This
// consumer does not itself register anything or query Windows Search.
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <searchapi.h>
#include <filterr.h>
#include <iostream>
#include <string>
#include <vector>
#include <sddl.h>

namespace {
constexpr CLSID kPrivateClsid = {
    0x987ad765, 0xd904, 0x4f10,
    {0xa6, 0x50, 0x91, 0x5d, 0x8f, 0x4c, 0x02, 0x87}};
constexpr CLSID kPrivateDocumentFilter = {
    0x44a4b197, 0x7237, 0x4958,
    {0xa1, 0xd0, 0x7c, 0xb2, 0x6e, 0x80, 0x41, 0x65}};
constexpr CLSID kPrivateContainerFilter = {
    0x5965eb3b, 0x44aa, 0x405f,
    {0x9c, 0x60, 0x13, 0x64, 0x07, 0x4d, 0xf2, 0x97}};
bool require(bool ok, const char* message) {
    if (!ok) std::cerr << "FAIL " << message << '\n';
    return ok;
}
}

int wmain() {
    HRESULT init = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(init)) return 2;
    ISearchProtocol* protocol = nullptr;
    HRESULT hr = CoCreateInstance(kPrivateClsid, nullptr, CLSCTX_INPROC_SERVER,
        __uuidof(ISearchProtocol), reinterpret_cast<void**>(&protocol));
    bool ok = require(hr == S_OK && protocol != nullptr,
                      "OS COM CoCreateInstance resolved private protocol CLSID");
    if (protocol) {
        ok &= require(protocol->Init(nullptr, nullptr, nullptr) == S_OK,
                      "Windows COM activated protocol initialization");
        IUrlAccessor* root = nullptr;
        ok &= require(protocol->CreateAccessor(L"stfixture-proto-test://root",
            nullptr, nullptr, nullptr, &root) == S_OK && root,
            "registered private protocol root accessor created");
        if (root) {
            ok &= require(root->IsDirectory() == S_OK,
                          "registered private protocol root recognized");
            ok &= require(protocol->CloseAccessor(root) == S_OK,
                          "root accessor closed");
        }
        IUrlAccessor* item = nullptr;
        ok &= require(protocol->CreateAccessor(
            L"stfixture-proto-test://root/document",
            nullptr, nullptr, nullptr, &item) == S_OK && item,
            "registered private protocol document accessor created");
        if (item) {
            DWORD size = 0;
            ok &= require(item->GetSecurityDescriptor(nullptr, 0, &size) ==
                HRESULT_FROM_WIN32(ERROR_INSUFFICIENT_BUFFER) &&
                size > sizeof(SECURITY_DESCRIPTOR),
                "registered private protocol item retains restricted ACL");
            ok &= require(protocol->CloseAccessor(item) == S_OK,
                          "document accessor closed");
        }

        // A per-user URL must resolve ONLY to this actual test identity.
        // No SID or file contents are printed in runner logs.
        HANDLE token = nullptr;
        bool token_ok = OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token) != FALSE;
        std::wstring scopedUrl;
        if (token_ok) {
            DWORD need = 0;
            GetTokenInformation(token, TokenUser, nullptr, 0, &need);
            token_ok = need >= sizeof(TOKEN_USER) && need <= 65536;
            if (token_ok) {
                std::vector<BYTE> bytes(need);
                token_ok = GetTokenInformation(token, TokenUser,
                    bytes.data(), need, &need) != FALSE;
                if (token_ok) {
                    LPWSTR sid = nullptr;
                    auto* user = reinterpret_cast<TOKEN_USER*>(bytes.data());
                    token_ok = ConvertSidToStringSidW(user->User.Sid, &sid) != FALSE;
                    if (token_ok) {
                        scopedUrl = std::wstring(L"stfixture-proto-test://{") +
                            sid + L"}/root/document";
                        LocalFree(sid);
                    }
                }
            }
            CloseHandle(token);
        }
        ok &= require(token_ok && !scopedUrl.empty(),
                      "get private test user SID without exposing it");
        if (token_ok) {
            IUrlAccessor* scoped = nullptr;
            hr = protocol->CreateAccessor(scopedUrl.c_str(), nullptr,
                                          nullptr, nullptr, &scoped);
            ok &= require(hr == S_OK && scoped != nullptr,
                          "own effective user SID URL accepted");
            if (scoped) {
                DWORD size = 999;
                ok &= require(scoped->GetSecurityDescriptor(nullptr, 0, &size) ==
                                  E_NOTIMPL && size == 0,
                              "SID-scoped item returns mandated E_NOTIMPL ACL");
                ok &= require(scoped->IsDirectory() == S_FALSE,
                              "user-scoped document recognized");
                ok &= require(protocol->CloseAccessor(scoped) == S_OK,
                              "user-scoped accessor closes");
            }
            IUrlAccessor* wrong = nullptr;
            hr = protocol->CreateAccessor(
                L"stfixture-proto-test://{S-1-5-18}/root/document",
                nullptr, nullptr, nullptr, &wrong);
            ok &= require(hr == HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND) &&
                          wrong == nullptr,
                          "SYSTEM SID URL cannot impersonate test user");
            if (wrong) wrong->Release();
        }
        protocol->Release();
    }
    for (const CLSID& id : {kPrivateDocumentFilter, kPrivateContainerFilter}) {
        IFilter* filter = nullptr;
        HRESULT activation = CoCreateInstance(id, nullptr, CLSCTX_INPROC_SERVER,
            IID_IFilter, reinterpret_cast<void**>(&filter));
        ok &= require(activation == S_OK && filter != nullptr,
                      "OS COM runtime loads separate private IFilter CLSID");
        if (filter) filter->Release();
    }
    CoUninitialize();
    if (ok) {
        std::cout << "NATIVE_SEARCH_HOSTED_VM_OS_COM_CLSID_ACTIVATION_PASS\n";
        return 0;
    }
    return 1;
}
