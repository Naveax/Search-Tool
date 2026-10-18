#pragma once
// Shared, registration-disabled fixture ACL identity policy.
// A no-user-SID protocol URL runs inside a system SearchProtocolHost;
// that process token MUST NOT silently become the document owner/reader.
// This deliberately does not implement Windows Search impersonation.
#include <windows.h>
#include <initializer_list>
#include <cstring>

inline HRESULT validate_fixture_acl_identity(PSID sid) noexcept {
    if (!sid || !IsValidSid(sid)) return HRESULT_FROM_WIN32(ERROR_INVALID_SID);
    // TokenUser is expected to identify a human test account. Refuse
    // well-known machine/service/group identities rather than granting
    // searchable ACL rights to a host service principal.
    for (auto kind : {
        WinLocalSystemSid, WinLocalServiceSid, WinNetworkServiceSid,
        WinServiceSid, WinWorldSid, WinAnonymousSid,
        WinBuiltinAdministratorsSid, WinBuiltinUsersSid,
        WinBuiltinGuestsSid, WinAuthenticatedUserSid
    }) {
        if (IsWellKnownSid(sid, kind)) return E_ACCESSDENIED;
    }
    // Service-specific identities such as NT SERVICE\\Name use
    // S-1-5-80-... and do not match the generic WinServiceSid group.
    const SID_IDENTIFIER_AUTHORITY nt = SECURITY_NT_AUTHORITY;
    const SID_IDENTIFIER_AUTHORITY* authority = GetSidIdentifierAuthority(sid);
    const UCHAR* sub_count = GetSidSubAuthorityCount(sid);
    if (authority && sub_count && *sub_count &&
        std::memcmp(authority, &nt, sizeof(nt)) == 0) {
        const DWORD base = *GetSidSubAuthority(sid, 0);
        if (base == 80 || base == 82) return E_ACCESSDENIED;
    }
    return S_OK;
}
