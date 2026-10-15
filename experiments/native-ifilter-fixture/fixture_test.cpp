// Executable local-only COM contract test. No CoRegisterClassObject,
// registration, search index query, taskbar interaction or shell API calls.
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <filter.h>
#include <filterr.h>
#include <objidl.h>
#include <iostream>
#include <string>
#include <vector>
#include <algorithm>
#include <atomic>

namespace {
constexpr CLSID kFixtureFilterClsid = {
    0x44a4b197, 0x7237, 0x4958, {0xa1, 0xd0, 0x7c, 0xb2, 0x6e, 0x80, 0x41, 0x65}};
using ClassGetter = HRESULT(__stdcall*)(REFCLSID, REFIID, void**);
using CanUnload = HRESULT(__stdcall*)();
using Registration = HRESULT(__stdcall*)();

bool check(bool condition, const char* label) {
    if (!condition) std::cerr << "FAIL " << label << '\n';
    return condition;
}

IStream* make_stream(const std::string& bytes) {
    IStream* stream = nullptr;
    if (FAILED(CreateStreamOnHGlobal(nullptr, TRUE, &stream))) return nullptr;
    ULONG written = 0;
    HRESULT hr = stream->Write(bytes.data(), static_cast<ULONG>(bytes.size()), &written);
    LARGE_INTEGER start{};
    if (FAILED(hr) || written != bytes.size() ||
        FAILED(stream->Seek(start, STREAM_SEEK_SET, nullptr))) {
        stream->Release();
        return nullptr;
    }
    return stream;
}


// A legal IStream may yield fewer bytes than requested with S_OK.
// Test partial UTF-8 reads and failures midway through a document.
class FragmentedStream final : public IStream {
public:
    enum class Mode { Normal, ReadError, ZeroProgress };
    FragmentedStream(IStream* inner, ULONG limit, Mode mode)
        : inner_(inner), limit_(limit), mode_(mode) { inner_->AddRef(); }
    ~FragmentedStream() { inner_->Release(); }
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** out) override {
        if (!out) return E_POINTER;
        *out = nullptr;
        if (iid != IID_IUnknown && iid != IID_ISequentialStream && iid != IID_IStream)
            return E_NOINTERFACE;
        *out = static_cast<IStream*>(this);
        AddRef();
        return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return ++refs_; }
    ULONG STDMETHODCALLTYPE Release() override {
        ULONG n = --refs_;
        if (!n) delete this;
        return n;
    }
    HRESULT STDMETHODCALLTYPE Read(void* data, ULONG request, ULONG* got) override {
        if (!got) return E_POINTER;
        *got = 0;
        const ULONG call = calls_++;
        if (call == 2 && mode_ == Mode::ReadError) return STG_E_READFAULT;
        if (call == 2 && mode_ == Mode::ZeroProgress) return S_OK;
        return inner_->Read(data, (std::min)(request, limit_), got);
    }
    HRESULT STDMETHODCALLTYPE Write(const void* b, ULONG n, ULONG* out) override {
        return inner_->Write(b, n, out);
    }
    HRESULT STDMETHODCALLTYPE Seek(LARGE_INTEGER move, DWORD origin,
                                   ULARGE_INTEGER* pos) override {
        return inner_->Seek(move, origin, pos);
    }
    HRESULT STDMETHODCALLTYPE SetSize(ULARGE_INTEGER size) override {
        return inner_->SetSize(size);
    }
    HRESULT STDMETHODCALLTYPE CopyTo(IStream* to, ULARGE_INTEGER n,
                                     ULARGE_INTEGER* read, ULARGE_INTEGER* written) override {
        return inner_->CopyTo(to, n, read, written);
    }
    HRESULT STDMETHODCALLTYPE Commit(DWORD flags) override { return inner_->Commit(flags); }
    HRESULT STDMETHODCALLTYPE Revert() override { return inner_->Revert(); }
    HRESULT STDMETHODCALLTYPE LockRegion(ULARGE_INTEGER start, ULARGE_INTEGER count,
                                         DWORD flags) override {
        return inner_->LockRegion(start, count, flags);
    }
    HRESULT STDMETHODCALLTYPE UnlockRegion(ULARGE_INTEGER start, ULARGE_INTEGER count,
                                           DWORD flags) override {
        return inner_->UnlockRegion(start, count, flags);
    }
    HRESULT STDMETHODCALLTYPE Stat(STATSTG* out, DWORD flags) override {
        return inner_->Stat(out, flags);
    }
    HRESULT STDMETHODCALLTYPE Clone(IStream** out) override { return inner_->Clone(out); }
private:
    std::atomic<ULONG> refs_{1};
    IStream* inner_;
    ULONG limit_;
    Mode mode_;
    ULONG calls_ = 0;
};

bool expect_stream(IFilter* filter, IPersistStream* persist,
                   const std::string& input, HRESULT wanted,
                   const std::wstring& expected = L"", ULONG fragment = 0,
                   FragmentedStream::Mode mode = FragmentedStream::Mode::Normal) {
    IStream* stream = make_stream(input);
    if (!check(stream != nullptr, "memory stream")) return false;
    IStream* tested = stream;
    if (fragment) {
        tested = new FragmentedStream(stream, fragment, mode);
        stream->Release();
    }
    HRESULT hr = persist->Load(tested);
    tested->Release();
    if (hr != wanted) {
        std::cerr << "Load mismatch bytes=" << input.size() << " fragment="
                  << fragment << " actual=0x" << std::hex
                  << static_cast<unsigned long>(hr) << " expected=0x"
                  << static_cast<unsigned long>(wanted) << std::dec << '\n';
        return check(false, "Load rejects/accepts fixture content");
    }
    if (FAILED(hr)) return true;
    ULONG flags = 999;
    if (!check(filter->Init(0, 0, nullptr, &flags) == S_OK && flags == 0,
               "IFilter::Init")) return false;
    STAT_CHUNK chunk{};
    HRESULT chunk_hr = filter->GetChunk(&chunk);
    if (expected.empty()) {
        return check(chunk_hr == FILTER_E_END_OF_CHUNKS, "empty chunk");
    }
    if (!check(chunk_hr == S_OK && chunk.flags == CHUNK_TEXT &&
               chunk.idChunk == 1, "first text chunk")) return false;
    if (!check(filter->GetValue(nullptr) == E_POINTER, "GetValue validates output")) {
        return false;
    }
    std::wstring result;
    while (true) {
        WCHAR buffer[4] = {};
        ULONG available = 3;
        HRESULT text_hr = filter->GetText(&available, buffer);
        if (text_hr == FILTER_E_NO_MORE_TEXT) break;
        if (FAILED(text_hr) || !available || available > 3) {
            return check(false, "chunk text streaming");
        }
        result.append(buffer, available);
        if (text_hr == FILTER_S_LAST_TEXT) break;
    }
    if (!check(result == expected, "full UTF-16 content preserved")) return false;
    if (!check(filter->GetChunk(&chunk) == FILTER_E_END_OF_CHUNKS,
               "exactly one content chunk")) return false;
    return true;
}
}

int wmain(int argc, wchar_t** argv) {
    if (argc != 2) {
        std::cerr << "FAIL supply build-output DLL path\n";
        return 1;
    }
    HRESULT init = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(init)) return 2;
    HMODULE dll = LoadLibraryExW(argv[1], nullptr, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR);
    if (!dll) {
        std::cerr << "FAIL unable to load local fixture DLL\n";
        CoUninitialize();
        return 3;
    }
    auto getter = reinterpret_cast<ClassGetter>(
        GetProcAddress(dll, "DllGetClassObject"));
    auto can_unload = reinterpret_cast<CanUnload>(
        GetProcAddress(dll, "DllCanUnloadNow"));
    auto reg = reinterpret_cast<Registration>(
        GetProcAddress(dll, "DllRegisterServer"));
    auto unreg = reinterpret_cast<Registration>(
        GetProcAddress(dll, "DllUnregisterServer"));
    bool ok = check(getter && can_unload && reg && unreg, "COM exports");
    if (ok) {
        ok &= check(reg() == E_ACCESSDENIED && unreg() == E_ACCESSDENIED,
                    "OS registration intentionally refused");
        ok &= check(can_unload() == S_OK, "DLL initially unloadable");
        IClassFactory* factory = nullptr;
        CLSID wrong{};
        ok &= check(getter(wrong, IID_IClassFactory,
                           reinterpret_cast<void**>(&factory)) ==
                    CLASS_E_CLASSNOTAVAILABLE, "unknown CLSID refused");
        ok &= check(getter(kFixtureFilterClsid, IID_IClassFactory,
                           reinterpret_cast<void**>(&factory)) == S_OK,
                    "class factory for private CLSID");
        if (factory) {
            ok &= check(can_unload() == S_FALSE, "live factory locked");
            IFilter* filter = nullptr;
            HRESULT hr = factory->CreateInstance(nullptr, IID_IFilter,
                                                  reinterpret_cast<void**>(&filter));
            ok &= check(hr == S_OK && filter != nullptr, "IFilter factory works");
            if (filter) {
                IPersistStream* persist = nullptr;
                hr = filter->QueryInterface(IID_IPersistStream,
                                             reinterpret_cast<void**>(&persist));
                ok &= check(hr == S_OK && persist != nullptr,
                            "required IPersistStream exposed");
                if (persist) {
                    CLSID actual{};
                    ok &= check(persist->GetClassID(&actual) == S_OK &&
                                actual == kFixtureFilterClsid, "persist CLSID");
                    ok &= expect_stream(filter, persist,
                         std::string(u8"STFIXTURE-TEXT-1\nRapor İstanbul Örnek Straße Γειά"),
                         S_OK, L"Rapor İstanbul Örnek Straße Γειά");
                    ok &= expect_stream(filter, persist,
                         "STFIXTURE-TEXT-1\n", S_OK);
                    ok &= expect_stream(filter, persist,
                         "UNTRUSTED\npayload", HRESULT_FROM_WIN32(ERROR_INVALID_DATA));
                    ok &= expect_stream(filter, persist,
                         std::string("STFIXTURE-TEXT-1\n") + "\xC3\x28",
                         HRESULT_FROM_WIN32(ERROR_NO_UNICODE_TRANSLATION));
                    ok &= expect_stream(filter, persist,
                         std::string("STFIXTURE-TEXT-1\n") + std::string(8192, 'x'),
                         HRESULT_FROM_WIN32(ERROR_FILE_TOO_LARGE));
                    const std::string unicode =
                         std::string(u8"STFIXTURE-TEXT-1\nRapor İstanbul Örnek Straße Γειά");
                     const std::wstring unicode_expected =
                         L"Rapor İstanbul Örnek Straße Γειά";
                     for (ULONG fragment : {1UL, 2UL, 3UL, 7UL, 255UL}) {
                         ok &= expect_stream(filter, persist, unicode,
                             S_OK, unicode_expected, fragment);
                     }
                     ok &= expect_stream(filter, persist, unicode,
                         STG_E_READFAULT, L"", 2,
                         FragmentedStream::Mode::ReadError);
                     ok &= expect_stream(filter, persist, unicode,
                         STG_E_READFAULT, L"", 2,
                         FragmentedStream::Mode::ZeroProgress);
                     constexpr size_t kHeaderSize = sizeof("STFIXTURE-TEXT-1\n") - 1;
                     ok &= expect_stream(filter, persist,
                         std::string("STFIXTURE-TEXT-1\n") +
                         std::string(8192 - kHeaderSize, 'x'),
                         S_OK, std::wstring(8192 - kHeaderSize, L'x'), 7);
                     ok &= expect_stream(filter, persist,
                         std::string("STFIXTURE-TEXT-1\n") +
                         std::string(8192, 'x'),
                         HRESULT_FROM_WIN32(ERROR_FILE_TOO_LARGE), L"", 1);
                     ULONG reset_flags = 0;
                     ok &= check(filter->Init(0, 0, nullptr, &reset_flags) ==
                         FILTER_E_ACCESS, "failed load leaves no indexed content");
                     ok &= expect_stream(filter, persist, unicode, S_OK,
                         unicode_expected, 3);
                     persist->Release();
                }
                filter->Release();
            }
            IFilter* aggregate = nullptr;
            ok &= check(factory->CreateInstance(
                reinterpret_cast<IUnknown*>(0x1), IID_IFilter,
                reinterpret_cast<void**>(&aggregate)) == CLASS_E_NOAGGREGATION,
                "COM aggregation refused");
            factory->Release();
        }
        ok &= check(can_unload() == S_OK, "all COM objects released");
    }
    FreeLibrary(dll);
    CoUninitialize();
    if (!ok) return 4;
    std::cout << "NATIVE_IFILTER_COM_FACTORY_STREAM_UTF8_REGISTRATION_DENIED_PASS\n";
    return 0;
}
