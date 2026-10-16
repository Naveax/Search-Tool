// Mutation is restricted to the private stfixture-proto-test root/rule.
// This binary is executed ONLY inside the guarded disposable Windows 11
// GitHub ARM64 guest script. Do NOT call this on user devices.
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <searchapi.h>
#include "fixture_sid_policy.h"
#include <sddl.h>
#include <string>
#include <vector>
#include <iostream>
#include <cwchar>

namespace {
constexpr wchar_t kRoot[] = L"stfixture-proto-test://root";
constexpr wchar_t kRule[] = L"stfixture-proto-test://root/*";
constexpr wchar_t kDirectDocument[] = L"stfixture-proto-test://root/document";

// Derive user-only test root from the effective Windows token. No SID is
// passed on the command line or emitted to public GitHub Actions logs.
std::wstring current_user_document() {
    HANDLE token = nullptr;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token)) return {};
    DWORD required = 0;
    GetTokenInformation(token, TokenUser, nullptr, 0, &required);
    if (required < sizeof(TOKEN_USER) || required > 65536) {
        CloseHandle(token); return {};
    }
    std::vector<BYTE> bytes(required);
    const bool got = GetTokenInformation(token, TokenUser, bytes.data(),
                                         required, &required) != FALSE;
    CloseHandle(token);
    if (!got) return {};
    auto* user = reinterpret_cast<TOKEN_USER*>(bytes.data());
    if (FAILED(validate_fixture_acl_identity(user->User.Sid))) return {};
    LPWSTR raw = nullptr;
    if (!ConvertSidToStringSidW(user->User.Sid, &raw)) return {};
    std::wstring result = std::wstring(L"stfixture-proto-test://{") +
        raw + L"}/root/document";
    LocalFree(raw);
    return result;
}
bool allow() {
    auto equals = [](const wchar_t* key, const wchar_t* expected) {
        wchar_t buf[128]{};
        return GetEnvironmentVariableW(key, buf, 128) != 0 &&
            std::wcscmp(buf, expected) == 0;
    };
    return equals(L"GITHUB_ACTIONS", L"true") &&
        equals(L"GITHUB_REPOSITORY", L"Naveax/Search-Tool") &&
        equals(L"RUNNER_ENVIRONMENT", L"github-hosted") &&
        equals(L"GITHUB_JOB", L"windows11-desktop-native-feasibility") &&
        equals(L"RUNNER_ARCH", L"ARM64");
}
int fail(const char* phase, HRESULT hr) {
    std::cerr << phase << "_HRESULT=0x" << std::hex <<
        static_cast<unsigned long>(hr) << std::dec << '\n';
    return 1;
}
}
int wmain(int argc, wchar_t** argv) {
    if (argc != 2 || (std::wcscmp(argv[1], L"add") &&
                       std::wcscmp(argv[1], L"remove") &&
                       std::wcscmp(argv[1], L"add-user") &&
                       std::wcscmp(argv[1], L"remove-user") &&
                       std::wcscmp(argv[1], L"add-document") &&
                       std::wcscmp(argv[1], L"remove-document") &&
                       std::wcscmp(argv[1], L"notify-document")) || !allow()) {
        return fail("ONLY_HOSTED_ARM_PRIVATE_CSM_MUTATION_ALLOWED", E_ACCESSDENIED);
    }
    const bool user_scoped = std::wcsstr(argv[1], L"-user") != nullptr;
    const bool direct_document = std::wcsstr(argv[1], L"-document") != nullptr;
    const bool add = std::wcscmp(argv[1], L"add") == 0 ||
                     std::wcscmp(argv[1], L"add-user") == 0 ||
                     std::wcscmp(argv[1], L"add-document") == 0;
    const std::wstring user_url = user_scoped ? current_user_document() : L"";
    if (user_scoped && user_url.empty()) {
        return fail("USER_SCOPED_ROOT_REQUIRES_REAL_NON_SERVICE_TOKEN",
                    E_ACCESSDENIED);
    }
    const wchar_t* root_url = user_scoped ? user_url.c_str()
                            : direct_document ? kDirectDocument : kRoot;
    const wchar_t* rule_url = user_scoped ? user_url.c_str()
                            : direct_document ? kDirectDocument : kRule;
    HRESULT hr = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(hr)) return fail("COM_INIT", hr);
    int ret = 0;
    ISearchManager* manager = nullptr;
    ISearchCatalogManager* catalog = nullptr;
    ISearchCrawlScopeManager* scope = nullptr;
    ISearchRoot* root = nullptr;
    do {
        hr = CoCreateInstance(__uuidof(CSearchManager), nullptr,
            CLSCTX_LOCAL_SERVER | CLSCTX_INPROC_SERVER,
            __uuidof(ISearchManager), reinterpret_cast<void**>(&manager));
        if (FAILED(hr) || !manager) { ret = fail("MANAGER", hr); break; }
        hr = manager->GetCatalog(L"SystemIndex", &catalog);
        if (FAILED(hr) || !catalog) { ret = fail("CATALOG", hr); break; }
        if (std::wcscmp(argv[1], L"notify-document") == 0) {
            // ONE fixed synthetic document, ONLY in the disposable guest.
            // Unlike ReindexSearchRoot this API returns a per-URL HRESULT
            // acknowledging acceptance into the indexing notification
            // queue. It is NOT evidence that a persisted row exists.
            ISearchPersistentItemsChangedSink* sink = nullptr;
            HRESULT acquire = catalog->GetPersistentItemsChangedSink(&sink);
            std::cout << "NATIVE_SEARCH_HOSTED_PRIVATE_NOTIFY_SINK_HRESULT=0x"
                << std::hex << static_cast<unsigned long>(acquire)
                << std::dec << '\n';
            if (SUCCEEDED(acquire) && sink) {
                SEARCH_ITEM_PERSISTENT_CHANGE change{};
                change.Change = SEARCH_CHANGE_ADD;
                change.URL = const_cast<LPWSTR>(kDirectDocument);
                change.OldURL = nullptr;
                change.Priority = SEARCH_NORMAL_PRIORITY;
                HRESULT completion = E_UNEXPECTED;
                const HRESULT call = sink->OnItemsChanged(
                    1, &change, &completion);
                std::cout << "NATIVE_SEARCH_HOSTED_PRIVATE_NOTIFY_CALL_HRESULT=0x"
                    << std::hex << static_cast<unsigned long>(call)
                    << std::dec << '\n';
                std::cout << "NATIVE_SEARCH_HOSTED_PRIVATE_NOTIFY_URL_ACK_HRESULT=0x"
                    << std::hex << static_cast<unsigned long>(completion)
                    << std::dec << '\n';
                sink->Release();
            }
            break;
        }
        hr = catalog->GetCrawlScopeManager(&scope);
        if (FAILED(hr) || !scope) { ret = fail("CRAWL_SCOPE", hr); break; }

        if (add) {
            hr = CoCreateInstance(__uuidof(CSearchRoot), nullptr,
                CLSCTX_INPROC_SERVER | CLSCTX_LOCAL_SERVER,
                __uuidof(ISearchRoot), reinterpret_cast<void**>(&root));
            if (FAILED(hr) || !root) { ret = fail("PRIVATE_ROOT_COM", hr); break; }
            if (FAILED(hr = root->put_RootURL(root_url)) ||
                FAILED(hr = root->put_IsHierarchical(
                    user_scoped || direct_document ? FALSE : TRUE)) ||
                FAILED(hr = root->put_ProvidesNotifications(FALSE)) ||
                FAILED(hr = root->put_UseNotificationsOnly(FALSE)) ||
                FAILED(hr = root->put_EnumerationDepth(
                    user_scoped || direct_document ? 0 : 1))) {
                ret = fail("CONFIGURE_PRIVATE_ROOT", hr); break;
            }
            hr = scope->AddRoot(root);
            if (FAILED(hr)) { ret = fail("ADD_PRIVATE_ROOT", hr); break; }
            // Unique rule cannot affect any file: or third-party store.
            hr = scope->AddUserScopeRule(rule_url, TRUE, FALSE, 0);
            if (FAILED(hr)) { ret = fail("ADD_PRIVATE_RULE", hr); break; }
            hr = scope->SaveAll();
            if (FAILED(hr)) { ret = fail("SAVE_PRIVATE_SCOPE", hr); break; }
            // A saved rule does not prove a URL is effectively included.
            // Query ONLY our exact private root, without enumerating user paths.
            BOOL included = FALSE;
            HRESULT inclusion = scope->IncludedInCrawlScope(root_url, &included);
            if (SUCCEEDED(inclusion) && included) {
                std::cout << "NATIVE_SEARCH_HOSTED_PRIVATE_SCOPE_EFFECTIVE_PASS\n";
            } else {
                std::cout << "NATIVE_SEARCH_HOSTED_PRIVATE_SCOPE_NOT_EFFECTIVE_HR=0x"
                    << std::hex << static_cast<unsigned long>(inclusion)
                    << std::dec << '\n';
            }
            // Microsoft's targeted re-index operation. Never use Reset(),
            // Reindex() or a wildcard matching existing Windows file: roots.
            // Failure is diagnostic, not an excuse to weaken registration.
            const HRESULT trigger = catalog->ReindexSearchRoot(root_url);
            if (SUCCEEDED(trigger)) {
                std::cout << "NATIVE_SEARCH_HOSTED_PRIVATE_ROOT_REINDEX_ACCEPTED\n";
            } else {
                std::cout << "NATIVE_SEARCH_HOSTED_PRIVATE_ROOT_REINDEX_REJECTED_HR=0x"
                    << std::hex << static_cast<unsigned long>(trigger)
                    << std::dec << '\n';
            }
            std::cout << "NATIVE_SEARCH_HOSTED_PRIVATE_CSM_ADD_PASS\n";
        } else {
            HRESULT r1 = scope->RemoveScopeRule(rule_url);
            HRESULT r2 = scope->RemoveRoot(root_url);
            HRESULT save = scope->SaveAll();
            if (FAILED(r1) || FAILED(r2) || FAILED(save)) {
                ret = fail("PRIVATE_CSM_REMOVE", FAILED(r1) ? r1 :
                           FAILED(r2) ? r2 : save);
                break;
            }
            std::cout << "NATIVE_SEARCH_HOSTED_PRIVATE_CSM_REMOVE_PASS\n";
        }
    } while (false);
    if (root) root->Release();
    if (scope) scope->Release();
    if (catalog) catalog->Release();
    if (manager) manager->Release();
    CoUninitialize();
    return ret;
}
