function Test-RegistryPath {
    <#
    .SYNOPSIS
        Validates that a registry path reference points to a valid target.
    .DESCRIPTION
        Checks if a file path, CLSID, or other reference stored in the registry
        actually exists and is accessible.
    .PARAMETER Path
        The path or reference to validate.
    .PARAMETER Type
        The type of reference (FilePath, CLSID, ProgId, TypeLib, etc.).
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory, ValueFromPipeline)]
        [AllowEmptyString()]
        [string]$Path,

        [Parameter()]
        [ValidateSet('FilePath', 'CLSID', 'ProgId', 'TypeLib', 'Interface', 'Extension', 'Auto')]
        [string]$Type = 'Auto'
    )

    process {
        if ([string]::IsNullOrWhiteSpace($Path)) {
            return @{ Valid = $false; Reason = 'Empty path' }
        }

        # Auto-detect type
        if ($Type -eq 'Auto') {
            $Type = switch -Regex ($Path) {
                '^\{[0-9A-Fa-f-]+\}$' { 'CLSID' }
                '^[A-Za-z]:\\' { 'FilePath' }
                '^\\\\' { 'FilePath' }  # UNC path
                '^\.' { 'Extension' }
                default { 'FilePath' }
            }
        }

        switch ($Type) {
            'FilePath' {
                # Expand environment variables
                $expandedPath = [Environment]::ExpandEnvironmentVariables($Path)

                # Handle paths with arguments
                $testPath = $expandedPath
                if ($expandedPath -match '^"([^"]+)"') {
                    $testPath = $Matches[1]
                }
                elseif ($expandedPath -match '^([^\s]+)') {
                    # Try to extract just the executable
                    $testPath = $Matches[1]
                }

                # Remove any remaining quotes
                $testPath = $testPath.Trim('"')

                if ([string]::IsNullOrWhiteSpace($testPath)) {
                    return @{ Valid = $false; Reason = 'Empty path after parsing' }
                }

                $exists = Test-Path -LiteralPath $testPath -ErrorAction SilentlyContinue

                return @{
                    Valid = $exists
                    ExpandedPath = $testPath
                    Reason = if ($exists) { 'File exists' } else { 'File not found' }
                }
            }

            'CLSID' {
                # Check if CLSID exists in registry
                $clsidPath = "HKLM:\SOFTWARE\Classes\CLSID\$Path"
                $clsidPath32 = "HKLM:\SOFTWARE\Classes\WOW6432Node\CLSID\$Path"

                $exists = (Test-Path $clsidPath) -or (Test-Path $clsidPath32)

                if ($exists) {
                    # Check if the InProcServer32 or LocalServer32 points to valid file
                    $serverPath = $null
                    foreach ($basePath in @($clsidPath, $clsidPath32)) {
                        if (Test-Path $basePath) {
                            $inproc = Join-Path $basePath 'InProcServer32'
                            $local = Join-Path $basePath 'LocalServer32'

                            if (Test-Path $inproc) {
                                $serverPath = (Get-ItemProperty -Path $inproc -ErrorAction SilentlyContinue).'(default)'
                            }
                            elseif (Test-Path $local) {
                                $serverPath = (Get-ItemProperty -Path $local -ErrorAction SilentlyContinue).'(default)'
                            }
                        }
                    }

                    if ($serverPath) {
                        $fileResult = Test-RegistryPath -Path $serverPath -Type FilePath
                        return @{
                            Valid = $fileResult.Valid
                            CLSIDPath = $clsidPath
                            ServerPath = $serverPath
                            Reason = if ($fileResult.Valid) { 'CLSID and server valid' } else { 'Server file not found' }
                        }
                    }
                }

                return @{
                    Valid = $exists
                    Reason = if ($exists) { 'CLSID exists' } else { 'CLSID not found' }
                }
            }

            'ProgId' {
                $progIdPath = "HKLM:\SOFTWARE\Classes\$Path"
                $exists = Test-Path $progIdPath

                return @{
                    Valid = $exists
                    Reason = if ($exists) { 'ProgId exists' } else { 'ProgId not found' }
                }
            }

            'TypeLib' {
                $typeLibPath = "HKLM:\SOFTWARE\Classes\TypeLib\$Path"
                $exists = Test-Path $typeLibPath

                return @{
                    Valid = $exists
                    Reason = if ($exists) { 'TypeLib exists' } else { 'TypeLib not found' }
                }
            }

            'Interface' {
                $interfacePath = "HKLM:\SOFTWARE\Classes\Interface\$Path"
                $exists = Test-Path $interfacePath

                return @{
                    Valid = $exists
                    Reason = if ($exists) { 'Interface exists' } else { 'Interface not found' }
                }
            }

            'Extension' {
                $extPath = "HKLM:\SOFTWARE\Classes\$Path"
                $exists = Test-Path $extPath

                return @{
                    Valid = $exists
                    Reason = if ($exists) { 'Extension registered' } else { 'Extension not found' }
                }
            }
        }
    }
}

function Find-OrphanedRegistryEntries {
    <#
    .SYNOPSIS
        Scans for orphaned registry entries in a specific category.
    .DESCRIPTION
        Looks for registry entries that reference non-existent files, CLSIDs, or other resources.
    .PARAMETER Category
        The category to scan.
    .PARAMETER MaxItems
        Maximum number of items to scan (for performance).
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [ValidateSet('InvalidPaths', 'MissingDLLs', 'OrphanedSoftware', 'InvalidExtensions', 'EmptyKeys')]
        [string]$Category,

        [Parameter()]
        [int]$MaxItems = 1000
    )

    $orphans = @()
    $scannedCount = 0

    switch ($Category) {
        'InvalidPaths' {
            # Check App Paths
            $appPathsKey = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths'
            if (Test-Path $appPathsKey) {
                $apps = Get-ChildItem -Path $appPathsKey -ErrorAction SilentlyContinue

                foreach ($app in $apps) {
                    if ($scannedCount -ge $MaxItems) { break }
                    $scannedCount++

                    try {
                        $defaultValue = (Get-ItemProperty -Path $app.PSPath -ErrorAction SilentlyContinue).'(default)'
                        if ($defaultValue) {
                            $result = Test-RegistryPath -Path $defaultValue -Type FilePath
                            if (-not $result.Valid) {
                                $orphans += [PSCustomObject]@{
                                    Category = 'InvalidPaths'
                                    Name = $app.PSChildName
                                    Path = $app.PSPath
                                    Value = $defaultValue
                                    Reason = $result.Reason
                                }
                            }
                        }
                    }
                    catch { }
                }
            }
        }

        'MissingDLLs' {
            # Check SharedDLLs
            $sharedDllKey = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\SharedDLLs'
            if (Test-Path $sharedDllKey) {
                try {
                    $props = Get-ItemProperty -Path $sharedDllKey -ErrorAction SilentlyContinue
                    foreach ($prop in $props.PSObject.Properties | Where-Object { $_.Name -notmatch '^PS' }) {
                        if ($scannedCount -ge $MaxItems) { break }
                        $scannedCount++

                        $result = Test-RegistryPath -Path $prop.Name -Type FilePath
                        if (-not $result.Valid) {
                            $orphans += [PSCustomObject]@{
                                Category = 'MissingDLLs'
                                Name = Split-Path $prop.Name -Leaf
                                Path = $sharedDllKey
                                Value = $prop.Name
                                ReferenceCount = $prop.Value
                                Reason = $result.Reason
                            }
                        }
                    }
                }
                catch { }
            }
        }

        'OrphanedSoftware' {
            # Check Uninstall keys for invalid paths
            $uninstallKeys = @(
                'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
                'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall',
                'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall'
            )

            foreach ($uninstallKey in $uninstallKeys) {
                if (-not (Test-Path $uninstallKey)) { continue }

                $entries = Get-ChildItem -Path $uninstallKey -ErrorAction SilentlyContinue

                foreach ($entry in $entries) {
                    if ($scannedCount -ge $MaxItems) { break }
                    $scannedCount++

                    try {
                        $props = Get-ItemProperty -Path $entry.PSPath -ErrorAction SilentlyContinue
                        $installLocation = $props.InstallLocation
                        $uninstallString = $props.UninstallString

                        # Check install location
                        if ($installLocation -and -not [string]::IsNullOrWhiteSpace($installLocation)) {
                            if (-not (Test-Path $installLocation -ErrorAction SilentlyContinue)) {
                                $orphans += [PSCustomObject]@{
                                    Category = 'OrphanedSoftware'
                                    Name = $props.DisplayName ?? $entry.PSChildName
                                    Path = $entry.PSPath
                                    Value = $installLocation
                                    Reason = 'Install location not found'
                                }
                            }
                        }
                    }
                    catch { }
                }
            }
        }

        'InvalidExtensions' {
            # Check file associations
            $classesRoot = 'HKLM:\SOFTWARE\Classes'
            $extensions = Get-ChildItem -Path $classesRoot -ErrorAction SilentlyContinue |
                Where-Object { $_.PSChildName -match '^\.' } |
                Select-Object -First $MaxItems

            foreach ($ext in $extensions) {
                $scannedCount++

                try {
                    $defaultValue = (Get-ItemProperty -Path $ext.PSPath -ErrorAction SilentlyContinue).'(default)'
                    if ($defaultValue) {
                        # Check if the ProgId exists
                        $progIdPath = Join-Path $classesRoot $defaultValue
                        if (-not (Test-Path $progIdPath)) {
                            $orphans += [PSCustomObject]@{
                                Category = 'InvalidExtensions'
                                Name = $ext.PSChildName
                                Path = $ext.PSPath
                                Value = $defaultValue
                                Reason = 'Associated ProgId not found'
                            }
                        }
                    }
                }
                catch { }
            }
        }

        'EmptyKeys' {
            # Find empty registry keys (no values and no subkeys)
            $searchPaths = @(
                'HKCU:\Software',
                'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall'
            )

            foreach ($searchPath in $searchPaths) {
                if (-not (Test-Path $searchPath)) { continue }

                $keys = Get-ChildItem -Path $searchPath -Recurse -ErrorAction SilentlyContinue |
                    Select-Object -First ($MaxItems - $scannedCount)

                foreach ($key in $keys) {
                    $scannedCount++

                    try {
                        $hasValues = (Get-ItemProperty -Path $key.PSPath -ErrorAction SilentlyContinue).PSObject.Properties |
                            Where-Object { $_.Name -notmatch '^PS' }
                        $hasSubkeys = (Get-ChildItem -Path $key.PSPath -ErrorAction SilentlyContinue).Count -gt 0

                        if (-not $hasValues -and -not $hasSubkeys) {
                            $orphans += [PSCustomObject]@{
                                Category = 'EmptyKeys'
                                Name = $key.PSChildName
                                Path = $key.PSPath
                                Value = $null
                                Reason = 'Empty registry key'
                            }
                        }
                    }
                    catch { }
                }
            }
        }
    }

    return @{
        Category = $Category
        ScannedCount = $scannedCount
        Orphans = $orphans
        OrphanCount = $orphans.Count
    }
}
