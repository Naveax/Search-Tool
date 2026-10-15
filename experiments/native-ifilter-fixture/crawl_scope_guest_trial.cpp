// Mutation is restricted to the private stfixture-proto-test root/rule.
// This binary is executed ONLY inside the guarded disposable Windows 11
// GitHub ARM64 guest script. Do NOT call this on user devices.
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <searchapi.h>
#include <iostream>
#include <cwchar>

namespace {
constexpr wchar_t kRoot[] = L"stfixture-proto-test://root";
constexpr wchar_t kRule[] = L"stfixture-proto-test://root/*";
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
                       std::wcscmp(argv[1], L"remove")) || !allow()) {
        return fail("ONLY_HOSTED_ARM_PRIVATE_CSM_MUTATION_ALLOWED", E_ACCESSDENIED);
    }
    const bool add = std::wcscmp(argv[1], L"add") == 0;
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
        hr = catalog->GetCrawlScopeManager(&scope);
        if (FAILED(hr) || !scope) { ret = fail("CRAWL_SCOPE", hr); break; }

        if (add) {
            hr = CoCreateInstance(__uuidof(CSearchRoot), nullptr,
                CLSCTX_INPROC_SERVER | CLSCTX_LOCAL_SERVER,
                __uuidof(ISearchRoot), reinterpret_cast<void**>(&root));
            if (FAILED(hr) || !root) { ret = fail("PRIVATE_ROOT_COM", hr); break; }
            if (FAILED(hr = root->put_RootURL(kRoot)) ||
                FAILED(hr = root->put_IsHierarchical(TRUE)) ||
                FAILED(hr = root->put_ProvidesNotifications(FALSE)) ||
                FAILED(hr = root->put_UseNotificationsOnly(FALSE)) ||
                FAILED(hr = root->put_EnumerationDepth(1))) {
                ret = fail("CONFIGURE_PRIVATE_ROOT", hr); break;
            }
            hr = scope->AddRoot(root);
            if (FAILED(hr)) { ret = fail("ADD_PRIVATE_ROOT", hr); break; }
            // Unique rule cannot affect any file: or third-party store.
            hr = scope->AddUserScopeRule(kRule, TRUE, FALSE, 0);
            if (FAILED(hr)) { ret = fail("ADD_PRIVATE_RULE", hr); break; }
            hr = scope->SaveAll();
            if (FAILED(hr)) { ret = fail("SAVE_PRIVATE_SCOPE", hr); break; }
            std::cout << "NATIVE_SEARCH_HOSTED_PRIVATE_CSM_ADD_PASS\n";
        } else {
            HRESULT r1 = scope->RemoveScopeRule(kRule);
            HRESULT r2 = scope->RemoveRoot(kRoot);
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
