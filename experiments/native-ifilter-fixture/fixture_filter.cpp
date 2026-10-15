// Development-only, registration-disabled IFilter for a synthetic custom
// .stfixture format. Never ship/register on an interactive Windows machine.
// The fixture is self-contained and implements IFilter + IPersistStream so a
// direct COM factory test can run without any OS Search configuration change.
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <filter.h>
#include <filterr.h>
#include <propidl.h>
#include <atomic>
#include <algorithm>
#include <new>
#include <string>

namespace {
// The private GUID has no shell or registered COM meaning until a test VM
// deliberately registers a compliant filter under a private file type.
constexpr CLSID kFilterClsid = {
    0x44a4b197, 0x7237, 0x4958, {0xa1, 0xd0, 0x7c, 0xb2, 0x6e, 0x80, 0x41, 0x65}};
constexpr GUID kStorageSet = {
    0xb725f130, 0x47ef, 0x101a, {0xa5, 0xf1, 0x02, 0x60, 0x8c, 0x9e, 0xeb, 0xac}};
constexpr PROPID kContentProperty = 19; // PID_STG_CONTENTS (Storage)
constexpr size_t kMaxFixtureBytes = 8192;
std::atomic<long> g_live_objects{0};
std::atomic<long> g_server_locks{0};

class FixtureFilter final : public IFilter, public IPersistStream {
public:
    FixtureFilter() noexcept { ++g_live_objects; }
    ~FixtureFilter() { --g_live_objects; }
    FixtureFilter(const FixtureFilter&) = delete;
    FixtureFilter& operator=(const FixtureFilter&) = delete;

    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** result) override {
        if (!result) return E_POINTER;
        *result = nullptr;
        if (iid == IID_IUnknown || iid == IID_IFilter) {
            *result = static_cast<IFilter*>(this);
        } else if (iid == IID_IPersist || iid == IID_IPersistStream) {
            *result = static_cast<IPersistStream*>(this);
        } else {
            return E_NOINTERFACE;
        }
        AddRef();
        return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return ++refs_; }
    ULONG STDMETHODCALLTYPE Release() override {
        ULONG remaining = --refs_;
        if (!remaining) delete this;
        return remaining;
    }

    // The indexer expects text chunks, not a custom view or a popup.
    SCODE STDMETHODCALLTYPE Init(ULONG, ULONG, const FULLPROPSPEC*, ULONG* flags) override {
        if (!flags) return E_POINTER;
        if (!loaded_) return FILTER_E_ACCESS;
        *flags = 0;
        chunk_seen_ = false;
        text_cursor_ = 0;
        return S_OK;
    }
    SCODE STDMETHODCALLTYPE GetChunk(STAT_CHUNK* chunk) override {
        if (!chunk) return E_POINTER;
        if (!loaded_) return FILTER_E_ACCESS;
        if (chunk_seen_ || contents_.empty()) return FILTER_E_END_OF_CHUNKS;
        chunk_seen_ = true;
        text_cursor_ = 0;
        *chunk = {};
        chunk->idChunk = 1;
        chunk->breakType = CHUNK_NO_BREAK;
        chunk->flags = CHUNK_TEXT;
        chunk->locale = MAKELCID(MAKELANGID(LANG_ENGLISH, SUBLANG_ENGLISH_US), SORT_DEFAULT);
        chunk->attribute.guidPropSet = kStorageSet;
        chunk->attribute.psProperty.ulKind = PRSPEC_PROPID;
        chunk->attribute.psProperty.propid = kContentProperty;
        return S_OK;
    }
    SCODE STDMETHODCALLTYPE GetText(ULONG* characters, WCHAR* destination) override {
        if (!characters || !destination) return E_POINTER;
        if (!chunk_seen_) return FILTER_E_NO_TEXT;
        if (!*characters) return E_INVALIDARG;
        if (text_cursor_ == contents_.size()) {
            *characters = 0;
            return FILTER_E_NO_MORE_TEXT;
        }
        const auto remaining = contents_.size() - text_cursor_;
        const ULONG copied = static_cast<ULONG>(
            (std::min)(static_cast<size_t>(*characters), remaining));
        std::copy_n(contents_.data() + text_cursor_, copied, destination);
        text_cursor_ += copied;
        *characters = copied;
        return text_cursor_ == contents_.size() ? FILTER_S_LAST_TEXT : S_OK;
    }
    SCODE STDMETHODCALLTYPE GetValue(PROPVARIANT** value) override {
        if (!value) return E_POINTER;
        *value = nullptr;
        return FILTER_E_NO_VALUES;
    }
    SCODE STDMETHODCALLTYPE BindRegion(FILTERREGION, REFIID, void** out) override {
        if (out) *out = nullptr;
        return E_NOTIMPL;
    }

    // Required Windows Search persistence path. It reads ONLY the IStream
    // provided by the caller; never opens arbitrary files or registry keys.
    HRESULT STDMETHODCALLTYPE GetClassID(CLSID* clsid) override {
        if (!clsid) return E_POINTER;
        *clsid = kFilterClsid;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE IsDirty() override { return S_FALSE; }
    HRESULT STDMETHODCALLTYPE Load(IStream* input) override {
        if (!input) return E_POINTER;
        loaded_ = false;
        contents_.clear();
        chunk_seen_ = false;
        text_cursor_ = 0;
        // Read one byte beyond the allowed maximum to reject long inputs.
        std::string buffer(kMaxFixtureBytes + 1, '\0');
        ULONG count = 0;
        HRESULT hr = input->Read(buffer.data(), static_cast<ULONG>(buffer.size()), &count);
        if (FAILED(hr)) return hr;
        if (count > kMaxFixtureBytes) return HRESULT_FROM_WIN32(ERROR_FILE_TOO_LARGE);
        buffer.resize(count);
        constexpr char kMagic[] = "STFIXTURE-TEXT-1\n";
        const size_t magic_size = sizeof(kMagic) - 1;
        if (buffer.size() < magic_size || buffer.compare(0, magic_size, kMagic) != 0) {
            return HRESULT_FROM_WIN32(ERROR_INVALID_DATA);
        }
        const char* body = buffer.data() + magic_size;
        const int body_len = static_cast<int>(buffer.size() - magic_size);
        if (body_len) {
            int width = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS,
                                             body, body_len, nullptr, 0);
            if (!width) return HRESULT_FROM_WIN32(GetLastError());
            contents_.resize(static_cast<size_t>(width));
            if (!MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, body, body_len,
                                    contents_.data(), width)) {
                contents_.clear();
                return HRESULT_FROM_WIN32(GetLastError());
            }
        }
        loaded_ = true;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE Save(IStream*, BOOL) override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE GetSizeMax(ULARGE_INTEGER* size) override {
        if (!size) return E_POINTER;
        size->QuadPart = 0;
        return E_NOTIMPL;
    }

private:
    std::atomic<ULONG> refs_{1};
    std::wstring contents_;
    size_t text_cursor_ = 0;
    bool loaded_ = false;
    bool chunk_seen_ = false;
};

class FixtureFactory final : public IClassFactory {
public:
    FixtureFactory() noexcept { ++g_live_objects; }
    ~FixtureFactory() { --g_live_objects; }
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** out) override {
        if (!out) return E_POINTER;
        *out = nullptr;
        if (iid != IID_IUnknown && iid != IID_IClassFactory) return E_NOINTERFACE;
        *out = static_cast<IClassFactory*>(this);
        AddRef();
        return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return ++refs_; }
    ULONG STDMETHODCALLTYPE Release() override {
        ULONG remaining = --refs_;
        if (!remaining) delete this;
        return remaining;
    }
    HRESULT STDMETHODCALLTYPE CreateInstance(IUnknown* outer, REFIID iid, void** out) override {
        if (!out) return E_POINTER;
        *out = nullptr;
        if (outer) return CLASS_E_NOAGGREGATION;
        auto* fixture = new (std::nothrow) FixtureFilter();
        if (!fixture) return E_OUTOFMEMORY;
        HRESULT hr = fixture->QueryInterface(iid, out);
        fixture->Release();
        return hr;
    }
    HRESULT STDMETHODCALLTYPE LockServer(BOOL lock) override {
        if (lock) {
            ++g_server_locks;
        } else {
            if (g_server_locks.load() <= 0) return E_UNEXPECTED;
            --g_server_locks;
        }
        return S_OK;
    }

private:
    std::atomic<ULONG> refs_{1};
};
} // namespace

STDAPI DllGetClassObject(REFCLSID clsid, REFIID iid, void** out) {
    if (!out) return E_POINTER;
    *out = nullptr;
    if (clsid != kFilterClsid) return CLASS_E_CLASSNOTAVAILABLE;
    auto* factory = new (std::nothrow) FixtureFactory();
    if (!factory) return E_OUTOFMEMORY;
    HRESULT hr = factory->QueryInterface(iid, out);
    factory->Release();
    return hr;
}

STDAPI DllCanUnloadNow() {
    return (g_live_objects.load() == 0 && g_server_locks.load() == 0) ? S_OK : S_FALSE;
}

// Refuse registration even if someone accidentally runs regsvr32. This is
// a DLL-under-test, NOT a usable OS integration or release component.
extern "C" __declspec(dllexport)
HRESULT __stdcall DllRegisterServer() { return E_ACCESSDENIED; }
extern "C" __declspec(dllexport)
HRESULT __stdcall DllUnregisterServer() { return E_ACCESSDENIED; }
