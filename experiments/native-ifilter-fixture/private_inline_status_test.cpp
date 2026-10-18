// Registrationless unit test for the guest-only Windows Search callback.
// Neither Windows Search COM nor any index/registry/service is accessed.
#include "private_inline_status.h"
#include <cstdio>

int main() {
    auto* site = new PrivateInlineStatus();
    ISearchNotifyInlineSite* iface = nullptr;
    if (site->QueryInterface(__uuidof(ISearchNotifyInlineSite),
            reinterpret_cast<void**>(&iface)) != S_OK || !iface)
        return 1;
    const SEARCH_ITEM_INDEXING_STATUS foreign{77, E_ACCESSDENIED};
    if (iface->OnItemIndexedStatusChange(SEARCH_INDEXING_PHASE_PERSISTED,
            1, const_cast<SEARCH_ITEM_INDEXING_STATUS*>(&foreign)) != S_OK)
        return 2;
    if (site->HasPersistedCallback()) return 3;
    site->SetTarget(42);
    if (iface->OnItemIndexedStatusChange(SEARCH_INDEXING_PHASE_GATHERER,
            1, const_cast<SEARCH_ITEM_INDEXING_STATUS*>(&foreign)) != S_OK)
        return 4;
    if (site->HasPersistedCallback()) return 5;
    SEARCH_ITEM_INDEXING_STATUS own{42, S_OK};
    if (iface->OnItemIndexedStatusChange(SEARCH_INDEXING_PHASE_GATHERER,
            1, &own) != S_OK)
        return 6;
    if (site->HasPersistedCallback()) return 7;
    if (iface->OnItemIndexedStatusChange(SEARCH_INDEXING_PHASE_QUERYABLE,
            1, &own) != S_OK)
        return 8;
    if (site->HasPersistedCallback()) return 9;
    if (iface->OnItemIndexedStatusChange(SEARCH_INDEXING_PHASE_PERSISTED,
            1, &own) != S_OK)
        return 10;
    if (!site->HasPersistedCallback()) return 11;
    if (iface->Release() != 1) return 12;
    if (site->Release() != 0) return 13;
    std::puts("NATIVE_PRIVATE_INLINE_STATUS_DOCID_FILTER_AND_PHASE_TEST_PASS");
    return 0;
}
