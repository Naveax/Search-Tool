// Windows Search Crawl Scope Manager: READ ONLY, no root/rule modification.
// Never print any other indexed paths or the contents of existing roots.
// Runtime COM query goes to the real SystemIndex catalog.
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <searchapi.h>
#include <iostream>
#include <cwchar>

namespace {
constexpr wchar_t kRoot[] = L"stfixture-proto-test://root";
constexpr wchar_t kDocument[] = L"stfixture-proto-test://root/document";
constexpr wchar_t kScheme[] = L"stfixture-proto-test:";
constexpr unsigned kMaxObserved = 8192;
bool has_private_prefix(const wchar_t* url) {
    return url && _wcsnicmp(url, kScheme, std::wcslen(kScheme)) == 0;
}
void result(const char* name, HRESULT hr) {
    std::cout << name << "_HR=0x" << std::hex <<
        static_cast<unsigned long>(hr) << std::dec << '\n';
}
}

int wmain() {
    HRESULT hr = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(hr)) { result("COM_INIT", hr); return 2; }
    ISearchManager* manager = nullptr;
    hr = CoCreateInstance(__uuidof(CSearchManager), nullptr,
        CLSCTX_LOCAL_SERVER | CLSCTX_INPROC_SERVER,
        __uuidof(ISearchManager), reinterpret_cast<void**>(&manager));
    if (FAILED(hr) || !manager) {
        result("SEARCH_MANAGER", hr); CoUninitialize(); return 3;
    }
    ISearchCatalogManager* catalog = nullptr;
    hr = manager->GetCatalog(L"SystemIndex", &catalog);
    manager->Release();
    if (FAILED(hr) || !catalog) {
        result("SYSTEMINDEX_CATALOG", hr); CoUninitialize(); return 4;
    }
    CatalogStatus catalogStatus = CATALOG_STATUS_IDLE;
    CatalogPausedReason pausedReason = CATALOG_PAUSED_REASON_NONE;
    HRESULT statusHr = catalog->GetCatalogStatus(&catalogStatus, &pausedReason);
    LONG incremental = 0;
    LONG notifications = 0;
    LONG highPriority = 0;
    HRESULT queueHr = catalog->NumberOfItemsToIndex(
        &incremental, &notifications, &highPriority);
    // Aggregate Windows Search catalog count, without URLs or user data.
    LONG totalItems = 0;
    HRESULT totalHr = catalog->NumberOfItems(&totalItems);
    // Only enum values and aggregate queue sizes; no URLBeingIndexed call
    // (which would expose a user's active indexed path).
    if (SUCCEEDED(statusHr)) {
        std::cout << "CATALOG_STATUS=" << static_cast<int>(catalogStatus)
                  << '\n';
        std::cout << "CATALOG_PAUSED_REASON=" << static_cast<int>(pausedReason)
                  << '\n';
    } else {
        result("CATALOG_STATUS", statusHr);
    }
    if (SUCCEEDED(totalHr) && totalItems >= 0) {
        std::cout << "CATALOG_TOTAL_ITEMS=" << totalItems << '\n';
    } else {
        result("CATALOG_TOTAL_ITEMS", FAILED(totalHr) ? totalHr : E_UNEXPECTED);
    }
    if (SUCCEEDED(queueHr)) {
        std::cout << "CATALOG_PENDING_INCREMENTAL=" << incremental << '\n';
        std::cout << "CATALOG_PENDING_NOTIFICATIONS=" << notifications << '\n';
        std::cout << "CATALOG_PENDING_HIGH_PRIORITY=" << highPriority << '\n';
    } else {
        result("CATALOG_QUEUE", queueHr);
    }
    ISearchCrawlScopeManager* scope = nullptr;
    hr = catalog->GetCrawlScopeManager(&scope);
    catalog->Release();
    if (FAILED(hr) || !scope) {
        result("CRAWL_SCOPE_INTERFACE", hr); CoUninitialize(); return 5;
    }

    bool ok = true;
    unsigned roots = 0, privateRoots = 0;
    IEnumSearchRoots* enumeration = nullptr;
    hr = scope->EnumerateRoots(&enumeration);
    if (FAILED(hr)) { result("ROOTS_ENUM", hr); ok = false; }
    else if (enumeration) {
        for (;;) {
            ISearchRoot* root = nullptr;
            ULONG fetched = 0;
            hr = enumeration->Next(1, &root, &fetched);
            if (FAILED(hr)) { result("ROOT_NEXT", hr); ok = false; }
            if (root) {
                LPWSTR url = nullptr;
                HRESULT read = root->get_RootURL(&url);
                if (FAILED(read)) { result("ROOT_READ", read); ok = false; }
                else if (has_private_prefix(url)) { ++privateRoots; }
                CoTaskMemFree(url);
                root->Release();
            }
            if (FAILED(hr) || !fetched) break;
            if (++roots >= kMaxObserved) {
                std::cout << "ROOT_ENUM_BOUNDED=1\n";
                ok = false;
                break;
            }
        }
        enumeration->Release();
    }

    unsigned rules = 0, privateRules = 0;
    IEnumSearchScopeRules* ruleEnumerator = nullptr;
    hr = scope->EnumerateScopeRules(&ruleEnumerator);
    if (FAILED(hr)) { result("RULES_ENUM", hr); ok = false; }
    else if (ruleEnumerator) {
        for (;;) {
            ISearchScopeRule* rule = nullptr;
            ULONG fetched = 0;
            hr = ruleEnumerator->Next(1, &rule, &fetched);
            if (FAILED(hr)) { result("RULE_NEXT", hr); ok = false; }
            if (rule) {
                LPWSTR pattern = nullptr;
                HRESULT read = rule->get_PatternOrURL(&pattern);
                if (FAILED(read)) { result("RULE_READ", read); ok = false; }
                else if (has_private_prefix(pattern)) { ++privateRules; }
                CoTaskMemFree(pattern);
                rule->Release();
            }
            if (FAILED(hr) || !fetched) break;
            if (++rules >= kMaxObserved) {
                std::cout << "RULE_ENUM_BOUNDED=1\n";
                ok = false;
                break;
            }
        }
        ruleEnumerator->Release();
    }
    BOOL rootIncluded = FALSE, documentIncluded = FALSE;
    HRESULT rootHr = scope->IncludedInCrawlScope(kRoot, &rootIncluded);
    HRESULT docHr = scope->IncludedInCrawlScope(kDocument, &documentIncluded);
    scope->Release();
    CoUninitialize();
    std::cout << "ROOTS_ENUMERATED=" << roots << '\n'
        << "RULES_ENUMERATED=" << rules << '\n'
        << "PRIVATE_ROOT_COUNT=" << privateRoots << '\n'
        << "PRIVATE_RULE_COUNT=" << privateRules << '\n'
        << "PRIVATE_ROOT_INCLUDED=" << (SUCCEEDED(rootHr) && rootIncluded ? 1 : 0) << '\n'
        << "PRIVATE_DOCUMENT_INCLUDED=" << (SUCCEEDED(docHr) && documentIncluded ? 1 : 0) << '\n';
    if (FAILED(rootHr)) result("PRIVATE_ROOT_SCOPE", rootHr);
    if (FAILED(docHr)) result("PRIVATE_DOCUMENT_SCOPE", docHr);
    if (!ok) return 8;
    std::cout << "NATIVE_SEARCH_CRAWL_SCOPE_READONLY_QUERY_PASS\n";
    return 0;
}
