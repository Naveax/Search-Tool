#pragma once
// Private, developer-only Windows Search fixture ETW invocation markers.
// Fixed ID only. NEVER emit URL, content, SID, token, PID or file path.
// Provider: {CBBAE291-F9D2-4E9D-A779-9DECE2129912}
#include <windows.h>
#include <evntprov.h>

inline void private_fixture_trace(unsigned short event_id) noexcept {
    if (!((event_id >= 100 && event_id <= 139) ||
          (event_id >= 200 && event_id <= 202))) return;
    static constexpr GUID provider = {
        0xcbbae291, 0xf9d2, 0x4e9d,
        {0xa7, 0x79, 0x9d, 0xec, 0xe2, 0x12, 0x99, 0x12}};
    REGHANDLE handle = 0;
    if (EventRegister(&provider, nullptr, nullptr, &handle) != ERROR_SUCCESS)
        return;
    EVENT_DESCRIPTOR descriptor{};
    descriptor.Id = event_id;
    descriptor.Level = 4; // Informational
    descriptor.Keyword = 1;
    // No event data descriptors: ETW records only standard metadata and ID.
    EventWrite(handle, &descriptor, 0, nullptr);
    EventUnregister(handle);
}

// Development-only exact HRESULT encoding in ETW EVENT HEADER fields,
// not in a payload. Task is the HRESULT upper 16 bits, Id is the lower
// 16 bits, and Keyword stays 1. No URL, path, SID or filter contents.
// Production code must not repurpose ETW task identifiers this way.
inline void private_fixture_trace_hresult(HRESULT error) noexcept {
    if (SUCCEEDED(error)) return;
    static constexpr GUID provider = {
        0xcbbae291, 0xf9d2, 0x4e9d,
        {0xa7, 0x79, 0x9d, 0xec, 0xe2, 0x12, 0x99, 0x12}};
    REGHANDLE handle = 0;
    if (EventRegister(&provider, nullptr, nullptr, &handle) != ERROR_SUCCESS)
        return;
    const DWORD bits = static_cast<DWORD>(error);
    EVENT_DESCRIPTOR descriptor{};
    descriptor.Id = static_cast<USHORT>(bits & 0xffffU);
    descriptor.Task = static_cast<USHORT>((bits >> 16) & 0xffffU);
    descriptor.Level = 4;
    descriptor.Keyword = 1;
    EventWrite(handle, &descriptor, 0, nullptr);
    EventUnregister(handle);
}
