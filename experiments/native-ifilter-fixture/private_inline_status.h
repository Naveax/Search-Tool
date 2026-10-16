#pragma once
// Guest-only Windows Search provider-managed completion observer.
// Records ONLY three indexing phases for one indexer-assigned synthetic DocID.
#include <windows.h>
#include <searchapi.h>
#include <atomic>
#include <iostream>

class PrivateInlineStatus final : public ISearchNotifyInlineSite {
public:
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** out) override {
        if (!out) return E_POINTER;
        *out = nullptr;
        if (iid != IID_IUnknown && iid != __uuidof(ISearchNotifyInlineSite))
            return E_NOINTERFACE;
        *out = static_cast<ISearchNotifyInlineSite*>(this);
        AddRef();
        return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return ++refs_; }
    ULONG STDMETHODCALLTYPE Release() override {
        const ULONG left = --refs_;
        if (!left) delete this;
        return left;
    }
    void SetTarget(DWORD doc) noexcept { target_doc_.store(doc); }
    HRESULT STDMETHODCALLTYPE OnItemIndexedStatusChange(
        SEARCH_INDEXING_PHASE phase, DWORD count,
        SEARCH_ITEM_INDEXING_STATUS* entries) override {
        if (count && !entries) return E_POINTER;
        const DWORD target = target_doc_.load();
        if (!target) { early_.fetch_add(1); return S_OK; }
        for (DWORD i=0; i<count; ++i) {
            if (entries[i].dwDocID != target) continue;
            switch (phase) {
                case SEARCH_INDEXING_PHASE_GATHERER:
                    gather_count_.fetch_add(1);
                    gather_hr_.store(entries[i].hrIndexingStatus);
                    break;
                case SEARCH_INDEXING_PHASE_QUERYABLE:
                    query_count_.fetch_add(1);
                    query_hr_.store(entries[i].hrIndexingStatus);
                    break;
                case SEARCH_INDEXING_PHASE_PERSISTED:
                    persist_count_.fetch_add(1);
                    persist_hr_.store(entries[i].hrIndexingStatus);
                    break;
                default: break;
            }
        }
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE OnCatalogStatusChange(REFGUID, REFGUID, DWORD) override {
        catalog_events_.fetch_add(1);
        return S_OK;
    }
    bool HasPersistedCallback() const noexcept {
        return persist_count_.load() != 0;
    }
    void Print() const {
        const auto code = [](HRESULT hr) {
            return static_cast<unsigned long>(hr);
        };
        std::cout << "NATIVE_SEARCH_HOSTED_PRIVATE_INLINE_GATHERER_COUNT="
                  << gather_count_.load() << '\n'
                  << "NATIVE_SEARCH_HOSTED_PRIVATE_INLINE_GATHERER_HRESULT=0x"
                  << std::hex << code(gather_hr_.load()) << std::dec << '\n'
                  << "NATIVE_SEARCH_HOSTED_PRIVATE_INLINE_QUERYABLE_COUNT="
                  << query_count_.load() << '\n'
                  << "NATIVE_SEARCH_HOSTED_PRIVATE_INLINE_QUERYABLE_HRESULT=0x"
                  << std::hex << code(query_hr_.load()) << std::dec << '\n'
                  << "NATIVE_SEARCH_HOSTED_PRIVATE_INLINE_PERSISTED_COUNT="
                  << persist_count_.load() << '\n'
                  << "NATIVE_SEARCH_HOSTED_PRIVATE_INLINE_PERSISTED_HRESULT=0x"
                  << std::hex << code(persist_hr_.load()) << std::dec << '\n'
                  << "NATIVE_SEARCH_HOSTED_PRIVATE_INLINE_EARLY_CALLBACKS="
                  << early_.load() << '\n'
                  << "NATIVE_SEARCH_HOSTED_PRIVATE_INLINE_CATALOG_CALLBACKS="
                  << catalog_events_.load() << '\n';
    }
private:
    std::atomic<ULONG> refs_{1};
    std::atomic<DWORD> target_doc_{0};
    std::atomic<unsigned long> gather_count_{0}, query_count_{0};
    std::atomic<unsigned long> persist_count_{0}, early_{0}, catalog_events_{0};
    std::atomic<HRESULT> gather_hr_{E_PENDING}, query_hr_{E_PENDING};
    std::atomic<HRESULT> persist_hr_{E_PENDING};
};
