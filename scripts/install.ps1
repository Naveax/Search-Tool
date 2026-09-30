[CmdletBinding()]
param(
    [ValidatePattern('^$|^[A-Za-z]:?$')]
    [string]$Drive = '',
    [string[]]$Drives = @(),
    [string]$SourceDir = '',
    [string]$InstallDir = [IO.Path]::Combine(
        [Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFiles),
        'Search Tool'
    ),
    [string]$DataDir = [IO.Path]::Combine(
        [Environment]::GetFolderPath([Environment+SpecialFolder]::CommonApplicationData),
        'SearchTool'
    ),
    [string]$ServiceName = 'SearchToolIndexer',
    [string]$StartupShortcutDir = '',
    [string]$ProgramsShortcutDir = '',
    [switch]$SkipInitialIndex,
    [switch]$SkipShortcut,
    [switch]$RecoverOnly,
    [switch]$RegistrySnapshotSelfTest
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$DefaultInstallDir = [IO.Path]::Combine(
    [Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFiles),
    'Search Tool'
)

if ([string]::IsNullOrWhiteSpace($ServiceName) -or
    $ServiceName.Length -gt 256 -or
    $ServiceName -match '[\\/"]') {
    throw "Invalid Windows service name: '$ServiceName'"
}
if ($ServiceName -ieq 'SearchToolIndexer' -and
    (-not [string]::IsNullOrWhiteSpace($StartupShortcutDir) -or
     -not [string]::IsNullOrWhiteSpace($ProgramsShortcutDir))) {
    throw 'Shortcut directory overrides are reserved for isolated validation services.'
}

$StageDir = "$InstallDir.new"
$BackupDir = "$InstallDir.old"
$MarkerPath = "$InstallDir.upgrade.json"
$ConfigPath = Join-Path $DataDir 'service.conf'
$ConfigBackupPath = "$ConfigPath.upgrade-backup"
$BinNames = @(
    'search-tool.exe',
    'search-tool-gui.exe',
    'search-tool-service.exe',
    'search-tool-worker.exe'
)

function Assert-Admin {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'Search Tool installation requires an elevated PowerShell session.'
    }
}

function Normalize-Drive([string]$Value) {
    if ([string]::IsNullOrWhiteSpace($Value)) { return $null }
    $letter = $Value.Substring(0, 1).ToUpperInvariant()
    if ($letter -notmatch '^[A-Z]$') { throw "Invalid drive: $Value" }
    return ($letter + ':')
}

function Assert-ExistingDataDirCompatible {
    $pointer = Join-Path $InstallDir 'service.conf.path'
    if (-not (Test-Path -LiteralPath $pointer -PathType Leaf)) { return }

    $raw = (Get-Content -Raw -LiteralPath $pointer).Trim().TrimStart([char]0xFEFF)
    if ([string]::IsNullOrWhiteSpace($raw)) {
        throw "Existing service.conf.path is empty: $pointer"
    }
    $actual = [IO.Path]::GetFullPath($raw)
    $expected = [IO.Path]::GetFullPath($ConfigPath)
    if ($actual -ine $expected) {
        throw "Existing install uses config '$actual'; rerun with the matching -DataDir before upgrading."
    }
}

function Get-TargetDrives {
    $requested = @()
    if ($Drives.Count -gt 0) {
        $requested = $Drives
    } elseif (-not [string]::IsNullOrWhiteSpace($Drive)) {
        $requested = @($Drive)
    }
    if ($requested.Count -gt 0) {
        return @($requested | ForEach-Object { Normalize-Drive $_ } | Sort-Object -Unique)
    }

    $detected = @(Get-Volume -ErrorAction Stop |
        Where-Object {
            $_.DriveLetter -and
            $_.FileSystemType -eq 'NTFS' -and
            $_.DriveType -eq 'Fixed'
        } |
        ForEach-Object { $_.DriveLetter.ToString().ToUpperInvariant() + ':' } |
        Sort-Object -Unique)
    if ($detected.Count -eq 0) {
        throw 'No visible fixed NTFS volumes were detected.'
    }
    return $detected
}

function Wait-ServiceDeletion {
    $deadline = (Get-Date).AddSeconds(15)
    do {
        if (-not (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue)) { return }
        Start-Sleep -Milliseconds 100
    } while ((Get-Date) -lt $deadline)
    throw "$ServiceName is still pending deletion."
}

function Stop-InstalledGui {
    if (-not (Test-Path -LiteralPath $InstallDir)) { return }
    $guiTarget = [IO.Path]::GetFullPath((Join-Path $InstallDir 'search-tool-gui.exe'))
    foreach ($process in @(Get-Process -Name 'search-tool-gui' -ErrorAction SilentlyContinue)) {
        try {
            if ($process.Path -and [IO.Path]::GetFullPath($process.Path) -ieq $guiTarget) {
                Stop-Process -Id $process.Id -Force -ErrorAction Stop
                $process.WaitForExit(10000) | Out-Null
            }
        } catch {
            throw "Failed to stop installed Search Tool GUI process $($process.Id): $($_.Exception.Message)"
        }
    }
}

function Remove-ServiceRegistration {
    $svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if (-not $svc) { return }
    if ($svc.Status -ne [System.ServiceProcess.ServiceControllerStatus]::Stopped) {
        Stop-Service -Name $ServiceName -Force -ErrorAction Stop
        $svc.WaitForStatus(
            [System.ServiceProcess.ServiceControllerStatus]::Stopped,
            [TimeSpan]::FromSeconds(20)
        )
    }
    $svc.Close()
    & sc.exe delete $ServiceName | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to delete $ServiceName service: $LASTEXITCODE"
    }
    Wait-ServiceDeletion
}

function Register-ExistingService([bool]$StartAfterRegister) {
    $serviceExe = Join-Path $InstallDir 'search-tool-service.exe'
    if (-not (Test-Path -LiteralPath $serviceExe)) {
        throw "Rollback service binary is missing: $serviceExe"
    }
    $quotedPath = '"' + $serviceExe + '" --service-name "' + $ServiceName + '"'
    $displayName = if ($ServiceName -eq 'SearchToolIndexer') {
        'Search Tool Indexer'
    } else {
        "Search Tool Indexer [$ServiceName]"
    }
    & sc.exe create $ServiceName binPath= $quotedPath start= auto DisplayName= $displayName | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to restore $ServiceName registration: $LASTEXITCODE"
    }
    if ($StartAfterRegister) {
        Start-Service -Name $ServiceName
        $svc = Get-Service -Name $ServiceName
        $svc.WaitForStatus(
            [System.ServiceProcess.ServiceControllerStatus]::Running,
            [TimeSpan]::FromSeconds(20)
        )
    }
}

function Write-UpgradePhase([System.Collections.IDictionary]$State, [string]$Phase) {
    $State['phase'] = $Phase
    $tmp = "$MarkerPath.tmp"
    $json = $State | ConvertTo-Json -Depth 8
    [IO.File]::WriteAllText($tmp, $json, [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $tmp -Destination $MarkerPath -Force
}

function Invoke-FaultPoint([string]$Name) {
    if ($env:SEARCH_TOOL_INSTALL_FAULT -eq $Name) {
        if ($env:SEARCH_TOOL_INSTALL_FAULT_MODE -eq 'exit') {
            [Environment]::Exit(197)
        }
        throw "Injected installer fault at $Name"
    }
}

function Remove-PathIfPresent([string]$Path) {
    if (Test-Path -LiteralPath $Path) {
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
}

function Test-DefaultProductionInstall {
    if ($ServiceName -ine 'SearchToolIndexer') { return $false }
    return [IO.Path]::GetFullPath($InstallDir) -ieq [IO.Path]::GetFullPath($DefaultInstallDir)
}

function Get-SearchToolStartupShortcutDir {
    if (-not [string]::IsNullOrWhiteSpace($StartupShortcutDir)) {
        return [IO.Path]::GetFullPath($StartupShortcutDir)
    }
    return [Environment]::GetFolderPath('Startup')
}

function Get-SearchToolProgramsShortcutDir {
    if (-not [string]::IsNullOrWhiteSpace($ProgramsShortcutDir)) {
        return [IO.Path]::GetFullPath($ProgramsShortcutDir)
    }
    return [Environment]::GetFolderPath('Programs')
}

function ConvertTo-RegistrySnapshotData($Value, [Microsoft.Win32.RegistryValueKind]$Kind) {
    switch ($Kind) {
        ([Microsoft.Win32.RegistryValueKind]::Binary) {
            return [Convert]::ToBase64String([byte[]]$Value)
        }
        ([Microsoft.Win32.RegistryValueKind]::None) {
            return [Convert]::ToBase64String([byte[]]$Value)
        }
        ([Microsoft.Win32.RegistryValueKind]::DWord) {
            return ([int32]$Value).ToString([Globalization.CultureInfo]::InvariantCulture)
        }
        ([Microsoft.Win32.RegistryValueKind]::QWord) {
            return ([int64]$Value).ToString([Globalization.CultureInfo]::InvariantCulture)
        }
        ([Microsoft.Win32.RegistryValueKind]::MultiString) {
            return @([string[]]$Value)
        }
        ([Microsoft.Win32.RegistryValueKind]::String) {
            return [string]$Value
        }
        ([Microsoft.Win32.RegistryValueKind]::ExpandString) {
            return [string]$Value
        }
        default {
            throw "Unsupported registry value kind in rollback snapshot: $Kind"
        }
    }
}

function ConvertFrom-RegistrySnapshotData($Snapshot) {
    $kindName = [string](Get-SnapshotValue $Snapshot 'kind')
    if ([string]::IsNullOrWhiteSpace($kindName)) {
        throw 'Registry rollback snapshot is missing a value kind.'
    }
    $kind = [Microsoft.Win32.RegistryValueKind][Enum]::Parse(
        [Microsoft.Win32.RegistryValueKind],
        $kindName,
        $true
    )
    $data = Get-SnapshotValue $Snapshot 'data'
    $value = switch ($kind) {
        ([Microsoft.Win32.RegistryValueKind]::Binary) {
            [Convert]::FromBase64String([string]$data)
            break
        }
        ([Microsoft.Win32.RegistryValueKind]::None) {
            [Convert]::FromBase64String([string]$data)
            break
        }
        ([Microsoft.Win32.RegistryValueKind]::DWord) {
            [int32]::Parse([string]$data, [Globalization.CultureInfo]::InvariantCulture)
            break
        }
        ([Microsoft.Win32.RegistryValueKind]::QWord) {
            [int64]::Parse([string]$data, [Globalization.CultureInfo]::InvariantCulture)
            break
        }
        ([Microsoft.Win32.RegistryValueKind]::MultiString) {
            [string[]]@($data | ForEach-Object { [string]$_ })
            break
        }
        ([Microsoft.Win32.RegistryValueKind]::String) {
            [string]$data
            break
        }
        ([Microsoft.Win32.RegistryValueKind]::ExpandString) {
            [string]$data
            break
        }
        default {
            throw "Unsupported registry value kind in rollback restore: $kind"
        }
    }
    [ordered]@{
        kind = $kind
        value = $value
    }
}

function Get-RegistryValueSnapshot([string]$Path, [string]$Name) {
    if (-not (Test-Path -LiteralPath $Path)) {
        return [ordered]@{
            path = $Path
            name = $Name
            exists = $false
        }
    }
    $key = Get-Item -LiteralPath $Path -ErrorAction Stop
    if ($key.GetValueNames() -notcontains $Name) {
        return [ordered]@{
            path = $Path
            name = $Name
            exists = $false
        }
    }
    $kind = $key.GetValueKind($Name)
    $value = $key.GetValue(
        $Name,
        $null,
        [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames
    )
    [ordered]@{
        path = $Path
        name = $Name
        exists = $true
        kind = $kind.ToString()
        data = ConvertTo-RegistrySnapshotData $value $kind
    }
}

function Get-RegistryTreeSnapshot([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) {
        return [ordered]@{
            path = $Path
            exists = $false
            keys = @()
        }
    }

    $root = Get-Item -LiteralPath $Path -ErrorAction Stop
    $nativeRoot = [string]$root.Name
    $keys = @()
    foreach ($key in @($root) + @(Get-ChildItem -LiteralPath $Path -Recurse -ErrorAction Stop)) {
        $nativeName = [string]$key.Name
        $relative = if ($nativeName.Length -eq $nativeRoot.Length) {
            ''
        } else {
            $nativeName.Substring($nativeRoot.Length + 1)
        }
        $values = @()
        foreach ($valueName in @($key.GetValueNames())) {
            $kind = $key.GetValueKind($valueName)
            $value = $key.GetValue(
                $valueName,
                $null,
                [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames
            )
            $values += [ordered]@{
                name = [string]$valueName
                kind = $kind.ToString()
                data = ConvertTo-RegistrySnapshotData $value $kind
            }
        }
        $keys += [ordered]@{
            relative_path = $relative
            values = @($values)
        }
    }

    [ordered]@{
        path = $Path
        exists = $true
        keys = @($keys)
    }
}

function Set-RegistryValueExact(
    [string]$Path,
    [string]$Name,
    $Value,
    [Microsoft.Win32.RegistryValueKind]$Kind
) {
    $hive = $null
    $subKey = $null
    if ($Path.StartsWith('HKLM:\', [StringComparison]::OrdinalIgnoreCase)) {
        $hive = [Microsoft.Win32.RegistryHive]::LocalMachine
        $subKey = $Path.Substring(('HKLM:\').Length)
    } elseif ($Path.StartsWith('HKCU:\', [StringComparison]::OrdinalIgnoreCase)) {
        $hive = [Microsoft.Win32.RegistryHive]::CurrentUser
        $subKey = $Path.Substring(('HKCU:\').Length)
    } else {
        throw "Unsupported registry path for exact restore: $Path"
    }
    if ([string]::IsNullOrWhiteSpace($subKey)) {
        throw "Refusing to write a registry hive root during exact restore: $Path"
    }

    $baseKey = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
        $hive,
        [Microsoft.Win32.RegistryView]::Default
    )
    try {
        $key = $baseKey.CreateSubKey($subKey, $true)
        if (-not $key) {
            throw "Failed to open registry key for exact restore: $Path"
        }
        try {
            $key.SetValue($Name, $Value, $Kind)
        } finally {
            $key.Dispose()
        }
    } finally {
        $baseKey.Dispose()
    }
}

function Restore-RegistryValueSnapshot($Snapshot) {
    $path = [string](Get-SnapshotValue $Snapshot 'path')
    $name = [string](Get-SnapshotValue $Snapshot 'name')
    if ([string]::IsNullOrWhiteSpace($path)) {
        throw 'Registry value rollback snapshot is missing its path.'
    }
    if (-not (Get-SnapshotFlag $Snapshot 'exists')) {
        Remove-ItemProperty -LiteralPath $path -Name $name -Force -ErrorAction SilentlyContinue
        return
    }

    New-Item -Path $path -Force | Out-Null
    $decoded = ConvertFrom-RegistrySnapshotData $Snapshot
    Set-RegistryValueExact -Path $path -Name $name -Value $decoded['value'] -Kind ([Microsoft.Win32.RegistryValueKind]$decoded['kind'])
}

function Restore-RegistryTreeSnapshot($Snapshot) {
    $path = [string](Get-SnapshotValue $Snapshot 'path')
    if ([string]::IsNullOrWhiteSpace($path)) {
        throw 'Registry tree rollback snapshot is missing its path.'
    }

    if (Test-Path -LiteralPath $path) {
        Remove-Item -LiteralPath $path -Recurse -Force -ErrorAction Stop
    }
    if (-not (Get-SnapshotFlag $Snapshot 'exists')) {
        return
    }

    New-Item -Path $path -Force | Out-Null
    foreach ($keySnapshot in @((Get-SnapshotValue $Snapshot 'keys'))) {
        $relative = [string](Get-SnapshotValue $keySnapshot 'relative_path')
        $keyPath = if ([string]::IsNullOrWhiteSpace($relative)) {
            $path
        } else {
            Join-Path $path $relative
        }
        New-Item -Path $keyPath -Force | Out-Null
        foreach ($valueSnapshot in @((Get-SnapshotValue $keySnapshot 'values'))) {
            $name = [string](Get-SnapshotValue $valueSnapshot 'name')
            $decoded = ConvertFrom-RegistrySnapshotData $valueSnapshot
            Set-RegistryValueExact -Path $keyPath -Name $name -Value $decoded['value'] -Kind ([Microsoft.Win32.RegistryValueKind]$decoded['kind'])
        }
    }
}

function Get-SearchToolUserArtifactSnapshot {
    $shortcutsTracked = -not [bool]$SkipShortcut
    $integrationTracked = Test-DefaultProductionInstall
    $startup = if ($shortcutsTracked) { Get-SearchToolStartupShortcutDir } else { $null }
    $programs = if ($shortcutsTracked) { Get-SearchToolProgramsShortcutDir } else { $null }
    $startupShortcut = if ($startup) { Join-Path $startup 'Search Tool.lnk' } else { $null }
    $programsShortcut = if ($programs) { Join-Path $programs 'Search Tool.lnk' } else { $null }
    $startupShortcutExists = [bool]($startupShortcut -and (Test-Path -LiteralPath $startupShortcut))
    $programsShortcutExists = [bool]($programsShortcut -and (Test-Path -LiteralPath $programsShortcut))

    $snapshot = [ordered]@{
        tracked = [bool]($shortcutsTracked -or $integrationTracked)
        shortcuts_tracked = $shortcutsTracked
        integration_tracked = $integrationTracked
        startup_shortcut_dir = $startup
        programs_shortcut_dir = $programs
        startup_shortcut = $startupShortcutExists
        startup_shortcut_base64 = if ($startupShortcutExists) {
            [Convert]::ToBase64String([IO.File]::ReadAllBytes($startupShortcut))
        } else { $null }
        programs_shortcut = $programsShortcutExists
        programs_shortcut_base64 = if ($programsShortcutExists) {
            [Convert]::ToBase64String([IO.File]::ReadAllBytes($programsShortcut))
        } else { $null }
        search_prog_id = $false
        searchtool_protocol = $false
        searchtool_root = $false
        capabilities = $false
        app_path = $false
        registered_application = $false
        search_open_with = $false
        directory_verb = $false
        directory_background_verb = $false
        drive_verb = $false
        registry_trees = $null
        registry_values = $null
    }

    if ($integrationTracked) {
        $snapshot['search_prog_id'] = [bool](Test-Path -LiteralPath 'HKLM:\SOFTWARE\Classes\SearchTool.Search')
        $snapshot['searchtool_protocol'] = [bool](Test-Path -LiteralPath 'HKLM:\SOFTWARE\Classes\searchtool')
        $snapshot['searchtool_root'] = [bool](Test-Path -LiteralPath 'HKLM:\SOFTWARE\SearchTool')
        $snapshot['capabilities'] = [bool](Test-Path -LiteralPath 'HKLM:\SOFTWARE\SearchTool\Capabilities')
        $snapshot['app_path'] = [bool](Test-Path -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\search-tool-gui.exe')
        $snapshot['registered_application'] = [bool](Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\RegisteredApplications' -Name 'Search Tool' -ErrorAction SilentlyContinue)
        $snapshot['search_open_with'] = [bool](Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Classes\search\OpenWithProgids' -Name 'SearchTool.Search' -ErrorAction SilentlyContinue)
        $snapshot['directory_verb'] = [bool](Test-Path -LiteralPath 'HKLM:\SOFTWARE\Classes\Directory\shell\SearchTool.SearchHere')
        $snapshot['directory_background_verb'] = [bool](Test-Path -LiteralPath 'HKLM:\SOFTWARE\Classes\Directory\Background\shell\SearchTool.SearchHere')
        $snapshot['drive_verb'] = [bool](Test-Path -LiteralPath 'HKLM:\SOFTWARE\Classes\Drive\shell\SearchTool.SearchHere')
        $snapshot['registry_trees'] = @(
            Get-RegistryTreeSnapshot 'HKLM:\SOFTWARE\Classes\SearchTool.Search'
            Get-RegistryTreeSnapshot 'HKLM:\SOFTWARE\Classes\searchtool'
            Get-RegistryTreeSnapshot 'HKLM:\SOFTWARE\SearchTool'
            Get-RegistryTreeSnapshot 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\search-tool-gui.exe'
            Get-RegistryTreeSnapshot 'HKLM:\SOFTWARE\Classes\Directory\shell\SearchTool.SearchHere'
            Get-RegistryTreeSnapshot 'HKLM:\SOFTWARE\Classes\Directory\Background\shell\SearchTool.SearchHere'
            Get-RegistryTreeSnapshot 'HKLM:\SOFTWARE\Classes\Drive\shell\SearchTool.SearchHere'
        )
        $snapshot['registry_values'] = @(
            Get-RegistryValueSnapshot 'HKLM:\SOFTWARE\RegisteredApplications' 'Search Tool'
            Get-RegistryValueSnapshot 'HKLM:\SOFTWARE\Classes\search\OpenWithProgids' 'SearchTool.Search'
        )
    }
    return $snapshot
}
function Get-SnapshotValue($Snapshot, [string]$Name) {
    if ($null -eq $Snapshot) { return $null }
    if ($Snapshot -is [System.Collections.IDictionary] -and $Snapshot.Contains($Name)) {
        return $Snapshot[$Name]
    }
    $property = $Snapshot.PSObject.Properties[$Name]
    if ($property) { return $property.Value }
    return $null
}

function Get-SnapshotFlag($Snapshot, [string]$Name) {
    return [bool](Get-SnapshotValue $Snapshot $Name)
}

function Invoke-RegistrySnapshotSelfTest {
    $provider = Get-PSDrive -Name HKCU -PSProvider Registry -ErrorAction SilentlyContinue
    if (-not $provider) {
        throw 'Registry snapshot self-test requires the Windows Registry provider.'
    }

    $testRoot = 'HKCU:\Software\SearchTool\RegistrySnapshotSelfTest\' + [Guid]::NewGuid().ToString('N')
    $absentPath = Join-Path $testRoot 'AbsentBefore'
    try {
        New-Item -Path $testRoot -Force | Out-Null
        Set-RegistryValueExact $testRoot '' 'legacy-default' ([Microsoft.Win32.RegistryValueKind]::String)
        Set-RegistryValueExact $testRoot 'Expand' '%TEMP%\SearchToolLegacy' ([Microsoft.Win32.RegistryValueKind]::ExpandString)
        Set-RegistryValueExact $testRoot 'Multi' ([string[]]@('alpha', 'beta')) ([Microsoft.Win32.RegistryValueKind]::MultiString)
        Set-RegistryValueExact $testRoot 'DWord' ([int32]-1) ([Microsoft.Win32.RegistryValueKind]::DWord)
        Set-RegistryValueExact $testRoot 'QWord' ([int64]::MaxValue) ([Microsoft.Win32.RegistryValueKind]::QWord)
        Set-RegistryValueExact $testRoot 'Binary' ([byte[]]@(0, 1, 2, 254, 255)) ([Microsoft.Win32.RegistryValueKind]::Binary)
        Set-RegistryValueExact $testRoot 'EmptyBinary' ([byte[]]@()) ([Microsoft.Win32.RegistryValueKind]::Binary)

        $childPath = Join-Path $testRoot 'Nested\Child'
        New-Item -Path $childPath -Force | Out-Null
        Set-RegistryValueExact $childPath 'ChildValue' 'nested-legacy' ([Microsoft.Win32.RegistryValueKind]::String)

        $payload = [ordered]@{
            tree = Get-RegistryTreeSnapshot $testRoot
            expand = Get-RegistryValueSnapshot $testRoot 'Expand'
            missing = Get-RegistryValueSnapshot $testRoot 'MissingValue'
            absent_tree = Get-RegistryTreeSnapshot $absentPath
        }
        # Upgrade markers persist this data as JSON. Exercise that exact
        # boundary so array/value-kind shape bugs cannot hide in memory-only tests.
        $payload = $payload | ConvertTo-Json -Depth 12 | ConvertFrom-Json

        Set-RegistryValueExact $testRoot '' 'mutated' ([Microsoft.Win32.RegistryValueKind]::String)
        Set-RegistryValueExact $testRoot 'Expand' 'mutated-expand' ([Microsoft.Win32.RegistryValueKind]::String)
        Set-RegistryValueExact $testRoot 'Multi' ([string[]]@('mutated')) ([Microsoft.Win32.RegistryValueKind]::MultiString)
        Set-RegistryValueExact $testRoot 'DWord' ([int32]7) ([Microsoft.Win32.RegistryValueKind]::DWord)
        Set-RegistryValueExact $testRoot 'QWord' ([int64]7) ([Microsoft.Win32.RegistryValueKind]::QWord)
        Set-RegistryValueExact $testRoot 'Binary' ([byte[]]@(9, 9)) ([Microsoft.Win32.RegistryValueKind]::Binary)
        Set-RegistryValueExact $testRoot 'EmptyBinary' ([byte[]]@(9)) ([Microsoft.Win32.RegistryValueKind]::Binary)
        Set-RegistryValueExact $testRoot 'Unexpected' 'remove-me' ([Microsoft.Win32.RegistryValueKind]::String)
        Set-RegistryValueExact $testRoot 'MissingValue' 'remove-me' ([Microsoft.Win32.RegistryValueKind]::String)
        Remove-Item -LiteralPath (Join-Path $testRoot 'Nested') -Recurse -Force
        New-Item -Path $absentPath -Force | Out-Null
        Set-RegistryValueExact $absentPath 'Unexpected' 'remove-me' ([Microsoft.Win32.RegistryValueKind]::String)

        Restore-RegistryTreeSnapshot $payload.tree
        Restore-RegistryValueSnapshot $payload.expand
        Restore-RegistryValueSnapshot $payload.missing
        Restore-RegistryTreeSnapshot $payload.absent_tree

        $root = Get-Item -LiteralPath $testRoot -ErrorAction Stop
        if ($root.GetValue('', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) -ne 'legacy-default' -or
            $root.GetValueKind('') -ne [Microsoft.Win32.RegistryValueKind]::String) {
            throw 'Registry snapshot self-test failed to restore the default String value.'
        }
        if ($root.GetValue('Expand', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) -ne '%TEMP%\SearchToolLegacy' -or
            $root.GetValueKind('Expand') -ne [Microsoft.Win32.RegistryValueKind]::ExpandString) {
            throw 'Registry snapshot self-test failed to restore ExpandString without expansion.'
        }
        $multi = [string[]]$root.GetValue('Multi', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        if (($multi -join ([char]0)) -ne ('alpha' + [char]0 + 'beta') -or
            $root.GetValueKind('Multi') -ne [Microsoft.Win32.RegistryValueKind]::MultiString) {
            throw 'Registry snapshot self-test failed to restore MultiString.'
        }
        if ([int32]$root.GetValue('DWord') -ne [int32]-1 -or
            $root.GetValueKind('DWord') -ne [Microsoft.Win32.RegistryValueKind]::DWord) {
            throw 'Registry snapshot self-test failed to restore DWord.'
        }
        if ([int64]$root.GetValue('QWord') -ne [int64]::MaxValue -or
            $root.GetValueKind('QWord') -ne [Microsoft.Win32.RegistryValueKind]::QWord) {
            throw 'Registry snapshot self-test failed to restore QWord.'
        }
        $binary = [byte[]]$root.GetValue('Binary')
        if (($binary -join ',') -ne '0,1,2,254,255' -or
            $root.GetValueKind('Binary') -ne [Microsoft.Win32.RegistryValueKind]::Binary) {
            throw 'Registry snapshot self-test failed to restore Binary.'
        }
        $emptyBinary = [byte[]]$root.GetValue('EmptyBinary')
        if ($emptyBinary.Length -ne 0 -or
            $root.GetValueKind('EmptyBinary') -ne [Microsoft.Win32.RegistryValueKind]::Binary) {
            throw 'Registry snapshot self-test failed to restore zero-length Binary.'
        }
        if ($root.GetValueNames() -contains 'Unexpected' -or $root.GetValueNames() -contains 'MissingValue') {
            throw 'Registry snapshot self-test failed to remove post-snapshot values.'
        }
        $restoredChild = Get-Item -LiteralPath $childPath -ErrorAction Stop
        if ($restoredChild.GetValue('ChildValue') -ne 'nested-legacy') {
            throw 'Registry snapshot self-test failed to restore nested keys.'
        }
        if (Test-Path -LiteralPath $absentPath) {
            throw 'Registry snapshot self-test failed to remove a tree that was absent before mutation.'
        }
    } finally {
        if (Test-Path -LiteralPath $testRoot) {
            Remove-Item -LiteralPath $testRoot -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

function Remove-NewSearchToolUserArtifacts($State) {
    $before = $null
    if ($State -is [System.Collections.IDictionary] -and $State.Contains('user_artifacts_before')) {
        $before = $State['user_artifacts_before']
    } else {
        $property = $State.PSObject.Properties['user_artifacts_before']
        if ($property) { $before = $property.Value }
    }
    if (-not (Get-SnapshotFlag $before 'tracked')) { return }

    $shortcutsTrackedValue = Get-SnapshotValue $before 'shortcuts_tracked'
    $shortcutsTracked = if ($null -eq $shortcutsTrackedValue) {
        Get-SnapshotFlag $before 'tracked'
    } else {
        [bool]$shortcutsTrackedValue
    }
    if ($shortcutsTracked) {
        $startup = [string](Get-SnapshotValue $before 'startup_shortcut_dir')
        if ([string]::IsNullOrWhiteSpace($startup)) { $startup = Get-SearchToolStartupShortcutDir }
        if ($startup) {
            $startupShortcut = Join-Path $startup 'Search Tool.lnk'
            if (-not (Get-SnapshotFlag $before 'startup_shortcut')) {
                Remove-Item -LiteralPath $startupShortcut -Force -ErrorAction SilentlyContinue
            } else {
                $startupBytes = [string](Get-SnapshotValue $before 'startup_shortcut_base64')
                if (-not [string]::IsNullOrWhiteSpace($startupBytes)) {
                    New-Item -ItemType Directory -Force -Path $startup | Out-Null
                    [IO.File]::WriteAllBytes(
                        $startupShortcut,
                        [Convert]::FromBase64String($startupBytes)
                    )
                }
            }
        }

        $programs = [string](Get-SnapshotValue $before 'programs_shortcut_dir')
        if ([string]::IsNullOrWhiteSpace($programs)) { $programs = Get-SearchToolProgramsShortcutDir }
        if ($programs) {
            $programsShortcut = Join-Path $programs 'Search Tool.lnk'
            if (-not (Get-SnapshotFlag $before 'programs_shortcut')) {
                Remove-Item -LiteralPath $programsShortcut -Force -ErrorAction SilentlyContinue
            } else {
                $programsBytes = [string](Get-SnapshotValue $before 'programs_shortcut_base64')
                if (-not [string]::IsNullOrWhiteSpace($programsBytes)) {
                    New-Item -ItemType Directory -Force -Path $programs | Out-Null
                    [IO.File]::WriteAllBytes(
                        $programsShortcut,
                        [Convert]::FromBase64String($programsBytes)
                    )
                }
            }
        }
    }

    $integrationTrackedValue = Get-SnapshotValue $before 'integration_tracked'
    $integrationTracked = if ($null -eq $integrationTrackedValue) {
        Get-SnapshotFlag $before 'tracked'
    } else {
        [bool]$integrationTrackedValue
    }
    if (-not $integrationTracked -or -not (Test-DefaultProductionInstall)) { return }

    $registryTrees = Get-SnapshotValue $before 'registry_trees'
    $registryValues = Get-SnapshotValue $before 'registry_values'
    if ($null -ne $registryTrees -and $null -ne $registryValues) {
        foreach ($treeSnapshot in @($registryTrees)) {
            Restore-RegistryTreeSnapshot $treeSnapshot
        }
        foreach ($valueSnapshot in @($registryValues)) {
            Restore-RegistryValueSnapshot $valueSnapshot
        }
        return
    }

    # Backward-compatible recovery for upgrade markers written before exact
    # registry snapshots were introduced.
    if (-not (Get-SnapshotFlag $before 'registered_application')) {
        Remove-ItemProperty -Path 'HKLM:\SOFTWARE\RegisteredApplications' -Name 'Search Tool' -Force -ErrorAction SilentlyContinue
    }
    if (-not (Get-SnapshotFlag $before 'search_open_with')) {
        Remove-ItemProperty -Path 'HKLM:\SOFTWARE\Classes\search\OpenWithProgids' -Name 'SearchTool.Search' -Force -ErrorAction SilentlyContinue
    }
    if (-not (Get-SnapshotFlag $before 'directory_verb')) {
        Remove-Item -LiteralPath 'HKLM:\SOFTWARE\Classes\Directory\shell\SearchTool.SearchHere' -Recurse -Force -ErrorAction SilentlyContinue
    }
    if (-not (Get-SnapshotFlag $before 'directory_background_verb')) {
        Remove-Item -LiteralPath 'HKLM:\SOFTWARE\Classes\Directory\Background\shell\SearchTool.SearchHere' -Recurse -Force -ErrorAction SilentlyContinue
    }
    if (-not (Get-SnapshotFlag $before 'drive_verb')) {
        Remove-Item -LiteralPath 'HKLM:\SOFTWARE\Classes\Drive\shell\SearchTool.SearchHere' -Recurse -Force -ErrorAction SilentlyContinue
    }
    if (-not (Get-SnapshotFlag $before 'app_path')) {
        Remove-Item -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\search-tool-gui.exe' -Recurse -Force -ErrorAction SilentlyContinue
    }
    if (-not (Get-SnapshotFlag $before 'capabilities')) {
        Remove-Item -LiteralPath 'HKLM:\SOFTWARE\SearchTool\Capabilities' -Recurse -Force -ErrorAction SilentlyContinue
    }
    if (-not (Get-SnapshotFlag $before 'searchtool_root')) {
        Remove-Item -LiteralPath 'HKLM:\SOFTWARE\SearchTool' -Force -ErrorAction SilentlyContinue
    }
    if (-not (Get-SnapshotFlag $before 'searchtool_protocol')) {
        Remove-Item -LiteralPath 'HKLM:\SOFTWARE\Classes\searchtool' -Recurse -Force -ErrorAction SilentlyContinue
    }
    if (-not (Get-SnapshotFlag $before 'search_prog_id')) {
        Remove-Item -LiteralPath 'HKLM:\SOFTWARE\Classes\SearchTool.Search' -Recurse -Force -ErrorAction SilentlyContinue
    }
}
function Restore-PreviousInstallation($State) {
    if ([string]$State.phase -eq 'staged') {
        Remove-PathIfPresent $StageDir
        if (Test-Path -LiteralPath $ConfigBackupPath) {
            Remove-Item -LiteralPath $ConfigBackupPath -Force
        }
        if (Test-Path -LiteralPath $MarkerPath) {
            Remove-Item -LiteralPath $MarkerPath -Force
        }
        return
    }

    Stop-InstalledGui
    Remove-ServiceRegistration
    Remove-NewSearchToolUserArtifacts $State

    if (Test-Path -LiteralPath $BackupDir) {
        Remove-PathIfPresent $InstallDir
        Move-Item -LiteralPath $BackupDir -Destination $InstallDir
    } elseif (-not [bool]$State.had_live_install) {
        Remove-PathIfPresent $InstallDir
    } elseif (-not (Test-Path -LiteralPath $InstallDir)) {
        throw 'Rollback cannot find the previous installation directory.'
    }

    if ([bool]$State.config_existed) {
        if (-not (Test-Path -LiteralPath $ConfigBackupPath)) {
            throw 'Rollback config backup is missing.'
        }
        New-Item -ItemType Directory -Force -Path $DataDir | Out-Null
        Copy-Item -LiteralPath $ConfigBackupPath -Destination $ConfigPath -Force
    } elseif (Test-Path -LiteralPath $ConfigPath) {
        Remove-Item -LiteralPath $ConfigPath -Force
    }

    if ([bool]$State.previous_service_exists) {
        Register-ExistingService ([bool]$State.previous_service_running)
    }

    Remove-PathIfPresent $StageDir
    if (Test-Path -LiteralPath $ConfigBackupPath) {
        Remove-Item -LiteralPath $ConfigBackupPath -Force
    }
    if (Test-Path -LiteralPath $MarkerPath) {
        Remove-Item -LiteralPath $MarkerPath -Force
    }
}

function Recover-InterruptedUpgrade {
    if (-not (Test-Path -LiteralPath $MarkerPath)) {
        if (Test-Path -LiteralPath $BackupDir) {
            throw "Orphaned upgrade backup exists without marker: $BackupDir"
        }
        Remove-PathIfPresent $StageDir
        return
    }

    $state = Get-Content -Raw -LiteralPath $MarkerPath | ConvertFrom-Json
    $recordedServiceName = if ($state.PSObject.Properties.Name -contains 'service_name' -and
        -not [string]::IsNullOrWhiteSpace([string]$state.service_name)) {
        [string]$state.service_name
    } else {
        'SearchToolIndexer'
    }
    if ($recordedServiceName -ine $ServiceName) {
        throw "Interrupted upgrade belongs to service '$recordedServiceName'; rerun with -ServiceName '$recordedServiceName'."
    }
    if ($state.phase -eq 'committed') {
        Remove-PathIfPresent $BackupDir
        Remove-PathIfPresent $StageDir
        if (Test-Path -LiteralPath $ConfigBackupPath) {
            Remove-Item -LiteralPath $ConfigBackupPath -Force
        }
        Remove-Item -LiteralPath $MarkerPath -Force
        return
    }
    Write-Warning "Recovering interrupted Search Tool upgrade at phase '$($state.phase)'."
    Restore-PreviousInstallation $state
}

function Validate-StagedPayload([string]$Stage) {
    foreach ($name in $BinNames) {
        $path = Join-Path $Stage $name
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "Staged binary missing: $path"
        }
        if ((Get-Item -LiteralPath $path).Length -le 0) {
            throw "Staged binary is empty: $path"
        }
    }
    $model = Join-Path $Stage 'models\tiny-intent-v1.stm'
    if (-not (Test-Path -LiteralPath $model -PathType Leaf)) {
        throw "Staged model missing: $model"
    }
    $version = @(& (Join-Path $Stage 'search-tool.exe') version 2>&1 | ForEach-Object { [string]$_ })
    if ($LASTEXITCODE -ne 0 -or -not ($version | Where-Object { $_ -match '^search-tool\s+' })) {
        throw "Staged CLI validation failed: $($version -join ' | ')"
    }
}

function New-SearchToolShortcuts([string]$IndexDir) {
    $shell = New-Object -ComObject WScript.Shell
    $gui = Join-Path $InstallDir 'search-tool-gui.exe'

    $startup = Get-SearchToolStartupShortcutDir
    if ($startup) {
        New-Item -ItemType Directory -Force -Path $startup | Out-Null
        $shortcutPath = Join-Path $startup 'Search Tool.lnk'
        $shortcut = $shell.CreateShortcut($shortcutPath)
        $shortcut.TargetPath = $gui
        $shortcut.Arguments = ('"{0}" --resident' -f $IndexDir)
        $shortcut.WorkingDirectory = $InstallDir
        $shortcut.Save()
    }

    $programs = Get-SearchToolProgramsShortcutDir
    if ($programs) {
        New-Item -ItemType Directory -Force -Path $programs | Out-Null
        $shortcutPath = Join-Path $programs 'Search Tool.lnk'
        $shortcut = $shell.CreateShortcut($shortcutPath)
        $shortcut.TargetPath = $gui
        $shortcut.Arguments = ('"{0}"' -f $IndexDir)
        $shortcut.WorkingDirectory = $InstallDir
        $shortcut.Save()
    }
}
function Register-SearchToolIntegration {
    # Global protocol registration is only appropriate for the production/default
    # installation. Validation installs deliberately use isolated paths/services
    # and must not mutate the user's Default Apps candidates.
    if ($ServiceName -ine 'SearchToolIndexer') { return }
    if ([IO.Path]::GetFullPath($InstallDir) -ine [IO.Path]::GetFullPath($DefaultInstallDir)) { return }

    $gui = Join-Path $InstallDir 'search-tool-gui.exe'
    $quotedCommand = '"' + $gui + '" --search-uri "%1"'

    $searchProgId = 'HKLM:\SOFTWARE\Classes\SearchTool.Search'
    New-Item -Path $searchProgId -Force | Out-Null
    Set-Item -Path $searchProgId -Value 'Search Tool'
    New-ItemProperty -Path $searchProgId -Name 'URL Protocol' -Value '' -PropertyType String -Force | Out-Null
    New-Item -Path (Join-Path $searchProgId 'DefaultIcon') -Force | Out-Null
    Set-Item -Path (Join-Path $searchProgId 'DefaultIcon') -Value ($gui + ',0')
    New-Item -Path (Join-Path $searchProgId 'shell\open\command') -Force | Out-Null
    Set-Item -Path (Join-Path $searchProgId 'shell\open\command') -Value $quotedCommand

    $privateProtocol = 'HKLM:\SOFTWARE\Classes\searchtool'
    New-Item -Path $privateProtocol -Force | Out-Null
    Set-Item -Path $privateProtocol -Value 'URL:Search Tool'
    New-ItemProperty -Path $privateProtocol -Name 'URL Protocol' -Value '' -PropertyType String -Force | Out-Null
    New-Item -Path (Join-Path $privateProtocol 'DefaultIcon') -Force | Out-Null
    Set-Item -Path (Join-Path $privateProtocol 'DefaultIcon') -Value ($gui + ',0')
    New-Item -Path (Join-Path $privateProtocol 'shell\open\command') -Force | Out-Null
    Set-Item -Path (Join-Path $privateProtocol 'shell\open\command') -Value $quotedCommand

    $capabilities = 'HKLM:\SOFTWARE\SearchTool\Capabilities'
    New-Item -Path $capabilities -Force | Out-Null
    New-ItemProperty -Path $capabilities -Name 'ApplicationName' -Value 'Search Tool' -PropertyType String -Force | Out-Null
    New-ItemProperty -Path $capabilities -Name 'ApplicationDescription' -Value 'Fast local Windows desktop search.' -PropertyType String -Force | Out-Null
    $urlAssociations = Join-Path $capabilities 'UrlAssociations'
    New-Item -Path $urlAssociations -Force | Out-Null
    New-ItemProperty -Path $urlAssociations -Name 'search' -Value 'SearchTool.Search' -PropertyType String -Force | Out-Null

    $registered = 'HKLM:\SOFTWARE\RegisteredApplications'
    New-Item -Path $registered -Force | Out-Null
    New-ItemProperty -Path $registered -Name 'Search Tool' -Value 'Software\SearchTool\Capabilities' -PropertyType String -Force | Out-Null

    $openWith = 'HKLM:\SOFTWARE\Classes\search\OpenWithProgids'
    New-Item -Path $openWith -Force | Out-Null
    New-ItemProperty -Path $openWith -Name 'SearchTool.Search' -Value ([byte[]]@()) -PropertyType Binary -Force | Out-Null

    $appPath = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\search-tool-gui.exe'
    New-Item -Path $appPath -Force | Out-Null
    Set-Item -Path $appPath -Value $gui
    New-ItemProperty -Path $appPath -Name 'Path' -Value $InstallDir -PropertyType String -Force | Out-Null

    # Classic unpackaged Win32 Explorer integration. On Windows 11 these static
    # shell verbs appear under Show more options; the native first-level menu
    # requires an app-identity/IExplorerCommand package extension. Keep this
    # registration lightweight, synchronous and shell-safe.
    $explorerVerbs = @(
        @{ Path = 'HKLM:\SOFTWARE\Classes\Directory\shell\SearchTool.SearchHere'; Argument = '%1' },
        @{ Path = 'HKLM:\SOFTWARE\Classes\Directory\Background\shell\SearchTool.SearchHere'; Argument = '%V' },
        @{ Path = 'HKLM:\SOFTWARE\Classes\Drive\shell\SearchTool.SearchHere'; Argument = '%1' }
    )
    foreach ($verb in $explorerVerbs) {
        New-Item -Path $verb.Path -Force | Out-Null
        Set-Item -Path $verb.Path -Value 'Search with Search Tool'
        New-ItemProperty -Path $verb.Path -Name 'Icon' -Value ($gui + ',0') -PropertyType String -Force | Out-Null
        New-ItemProperty -Path $verb.Path -Name 'MultiSelectModel' -Value 'Single' -PropertyType String -Force | Out-Null
        $command = Join-Path $verb.Path 'command'
        New-Item -Path $command -Force | Out-Null
        Set-Item -Path $command -Value ('"' + $gui + '" --scope "' + $verb.Argument + '"')
    }
}

if ($RegistrySnapshotSelfTest) {
    Invoke-RegistrySnapshotSelfTest
    Write-Host 'registry_snapshot_self_test=PASS'
    return
}

Assert-Admin
Assert-ExistingDataDirCompatible
Recover-InterruptedUpgrade
if ($RecoverOnly) {
    Write-Host 'Search Tool interrupted-upgrade recovery complete.'
    return
}

if ([string]::IsNullOrWhiteSpace($SourceDir)) {
    $portableCli = Join-Path $PSScriptRoot 'search-tool.exe'
    if (Test-Path -LiteralPath $portableCli) {
        $SourceDir = $PSScriptRoot
    } else {
        $SourceDir = (Resolve-Path (Join-Path $PSScriptRoot '..\target\release')).Path
    }
}

$TargetDrives = @(Get-TargetDrives)
$IndexDir = Join-Path $DataDir 'index'
$parent = Split-Path -Parent $InstallDir
if ([string]::IsNullOrWhiteSpace($parent)) {
    throw "InstallDir must have a parent directory: $InstallDir"
}
New-Item -ItemType Directory -Force -Path $parent, $DataDir, $IndexDir | Out-Null

Remove-PathIfPresent $StageDir
New-Item -ItemType Directory -Force -Path $StageDir | Out-Null
foreach ($name in $BinNames) {
    $source = Join-Path $SourceDir $name
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
        throw "Missing release binary: $source"
    }
    Copy-Item -LiteralPath $source -Destination (Join-Path $StageDir $name)
}

$modelSource = Join-Path $PSScriptRoot 'models\tiny-intent-v1.stm'
if (-not (Test-Path -LiteralPath $modelSource)) {
    $modelSource = Join-Path $PSScriptRoot '..\models\tiny-intent-v1.stm'
}
if (-not (Test-Path -LiteralPath $modelSource -PathType Leaf)) {
    throw "Missing required tiny intent model: $modelSource"
}
$modelDir = Join-Path $StageDir 'models'
New-Item -ItemType Directory -Force -Path $modelDir | Out-Null
Copy-Item -LiteralPath $modelSource -Destination (Join-Path $modelDir 'tiny-intent-v1.stm')

$configPointer = Join-Path $StageDir 'service.conf.path'
[IO.File]::WriteAllText($configPointer, $ConfigPath, [Text.UTF8Encoding]::new($false))
Validate-StagedPayload $StageDir

$previousService = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
$userArtifactsBefore = Get-SearchToolUserArtifactSnapshot
$state = [ordered]@{
    version = 2
    phase = 'staged'
    service_name = $ServiceName
    created_utc = [DateTime]::UtcNow.ToString('o')
    had_live_install = [bool](Test-Path -LiteralPath $InstallDir)
    previous_service_exists = [bool]$previousService
    previous_service_running = [bool]($previousService -and $previousService.Status -eq 'Running')
    config_existed = [bool](Test-Path -LiteralPath $ConfigPath)
    install_dir = $InstallDir
    stage_dir = $StageDir
    backup_dir = $BackupDir
    data_dir = $DataDir
    index_dir = $IndexDir
    target_drives = @($TargetDrives)
    user_artifacts_before = $userArtifactsBefore
}
if ($state.config_existed) {
    Copy-Item -LiteralPath $ConfigPath -Destination $ConfigBackupPath -Force
} elseif (Test-Path -LiteralPath $ConfigBackupPath) {
    Remove-Item -LiteralPath $ConfigBackupPath -Force
}
Write-UpgradePhase $state 'staged'

try {
    Invoke-FaultPoint 'after-stage-validated'

    $stagedCli = Join-Path $StageDir 'search-tool.exe'
    foreach ($targetDrive in $TargetDrives) {
        $letter = $targetDrive.Substring(0, 1)
        $indexPath = Join-Path $IndexDir ($letter + '.stidx')
        if (Test-Path -LiteralPath $indexPath) {
            Write-Host "Preserving existing $targetDrive index: $indexPath"
            continue
        }
        if ($SkipInitialIndex) {
            Write-Warning "Initial index skipped and no index exists at $indexPath."
            continue
        }
        Write-Host "Building initial $targetDrive index..."
        & $stagedCli index $targetDrive $indexPath
        if ($LASTEXITCODE -ne 0) {
            throw "Initial index build for $targetDrive failed with exit code $LASTEXITCODE"
        }
    }

    Stop-InstalledGui
    Remove-ServiceRegistration
    Write-UpgradePhase $state 'old-service-removed'
    Invoke-FaultPoint 'after-service-removed'

    Remove-PathIfPresent $BackupDir
    if (Test-Path -LiteralPath $InstallDir) {
        Move-Item -LiteralPath $InstallDir -Destination $BackupDir
    }
    Write-UpgradePhase $state 'live-renamed'
    Invoke-FaultPoint 'after-live-renamed'

    Move-Item -LiteralPath $StageDir -Destination $InstallDir
    Write-UpgradePhase $state 'new-published'
    Invoke-FaultPoint 'after-new-published'

    $service = Join-Path $InstallDir 'search-tool-service.exe'
    $driveList = (($TargetDrives | ForEach-Object { $_.Substring(0, 1) }) -join ',')
    & $service --service-name $ServiceName --install-multi $IndexDir $driveList
    if ($LASTEXITCODE -ne 0) {
        throw "Service installation failed with exit code $LASTEXITCODE"
    }
    Write-UpgradePhase $state 'service-installed'
    Invoke-FaultPoint 'after-service-installed'
    Invoke-FaultPoint 'before-service-start'

    & $service --service-name $ServiceName --start
    if ($LASTEXITCODE -ne 0) {
        throw "Service start failed with exit code $LASTEXITCODE"
    }
    Write-UpgradePhase $state 'service-started'
    Invoke-FaultPoint 'after-service-started'

    Invoke-FaultPoint 'before-user-artifacts'
    if (-not $SkipShortcut) {
        New-SearchToolShortcuts $IndexDir
        Invoke-FaultPoint 'after-shortcuts'
    }
    Register-SearchToolIntegration
    Invoke-FaultPoint 'after-integration'
    Write-UpgradePhase $state 'committed'
} catch {
    $installError = $_
    try {
        Restore-PreviousInstallation $state
    } catch {
        throw "Upgrade failed: $($installError.Exception.Message); rollback also failed: $($_.Exception.Message). Recovery marker preserved at $MarkerPath"
    }
    throw $installError
}

try {
    Remove-PathIfPresent $BackupDir
    if (Test-Path -LiteralPath $ConfigBackupPath) {
        Remove-Item -LiteralPath $ConfigBackupPath -Force
    }
    Remove-Item -LiteralPath $MarkerPath -Force
} catch {
    Write-Warning "Install committed but cleanup is deferred to the next installer run: $($_.Exception.Message)"
}

Write-Host 'Search Tool installed.'
Write-Host "InstallDir=$InstallDir"
Write-Host "IndexDir=$IndexDir"
Write-Host "Volumes=$($TargetDrives -join ',')"
