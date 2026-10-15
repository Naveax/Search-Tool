// Fully in-process protocol-to-stream COM contract test. Never invokes
// CoCreateInstance or uses registry, Windows Search, Start, or shell services.
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <searchapi.h>
#include <filterr.h>
#include <objidl.h>
#include <iostream>
#include <string>
#include <vector>
#include <cstring>

namespace {
constexpr CLSID kProtocolClsid = {
    0x987ad765, 0xd904, 0x4f10, {0xa6, 0x50, 0x91, 0x5d, 0x8f, 0x4c, 0x02, 0x87}};
constexpr wchar_t kUrl[] = L"stfixture-proto-test://root/document";
constexpr CLSID kFilterClsid = {
    0x44a4b197, 0x7237, 0x4958, {0xa1, 0xd0, 0x7c, 0xb2, 0x6e, 0x80, 0x41, 0x65}};
using ClassGetter = HRESULT(__stdcall*)(REFCLSID, REFIID, void**);
using CanUnload = HRESULT(__stdcall*)();
using Registration = HRESULT(__stdcall*)();

bool check(bool condition, const char* message) {
    if (!condition) std::cerr << "FAIL " << message << '\n';
    return condition;
}
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
                    ok &= check(accessor->GetSecurityDescriptor(nullptr, 0,
                                &needed) == E_NOTIMPL,
                                "real ACL integration not claimed");
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
    std::cout << "NATIVE_PROTOCOL_TO_IFILTER_END_TO_END_NO_REGISTRATION_PASS\n";
    return 0;
}
