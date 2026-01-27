function Get-WinMoleConfig {
    <#
    .SYNOPSIS
        Gets the WinMole configuration.
    .DESCRIPTION
        Loads configuration from the defaults file and merges with any user overrides.
    .PARAMETER Key
        Specific configuration key to retrieve.
    #>
    [CmdletBinding()]
    param(
        [Parameter()]
        [string]$Key
    )

    # Default configuration
    $defaultConfig = @{
        # Cleanup settings
        Cleanup = @{
            DefaultCategories = @('User', 'Browser', 'Cache')
            SkipConfirmationForDryRun = $true
            MaxFilesToShow = 50
        }

        # Disk analysis settings
        DiskAnalysis = @{
            DefaultDepth = 3
            TopNItems = 10
            MinSizeToShow = 1MB
        }

        # Status monitor settings
        Status = @{
            RefreshIntervalSeconds = 2
            ShowHealthScore = $true
            TopProcessCount = 5
        }

        # Developer cleanup settings
        DevCleanup = @{
            DefaultArtifactTypes = @('node_modules', 'bin', 'obj', '__pycache__')
            DefaultOlderThanDays = 30
            MaxDepth = 10
        }

        # Winget settings
        Winget = @{
            AutoUpdateCheck = $true
            BatchSize = 10
            SkipPinned = $true
        }

        # Registry cleaner settings
        Registry = @{
            DefaultCategories = @('InvalidPaths', 'MissingDLLs', 'OrphanedSoftware')
            BackupBeforeClean = $true
            MaxItemsPerCategory = 500
            SafeMode = $true  # Only remove confirmed orphans
        }

        # Startup optimizer settings
        Startup = @{
            ShowServices = $false  # Services can be overwhelming
            ShowImpact = $true
            CategorizeByPublisher = $true
        }

        # UI settings
        UI = @{
            UseColors = $true
            UseUnicodeChars = $true
            ProgressBarWidth = 20
            TableBorders = $true
        }

        # Whitelist - paths that should never be cleaned
        Whitelist = @{
            Paths = @(
                '$env:USERPROFILE\Documents',
                '$env:USERPROFILE\Desktop',
                '$env:USERPROFILE\Pictures',
                '$env:USERPROFILE\Music',
                '$env:USERPROFILE\Videos'
            )
            Patterns = @(
                '*.dll',
                '*.sys',
                'ntuser.*'
            )
        }

        # Blacklist - patterns that are always safe to clean
        Blacklist = @{
            Patterns = @(
                '*.tmp',
                '*.temp',
                '~*',
                'Thumbs.db',
                '*.log'
            )
        }
    }

    # Try to load custom config
    $configPath = $script:WinMoleConfigPath
    $userConfigPath = Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'WinMole\config.json'

    $customConfig = @{}

    # Load module default config
    if ($configPath -and (Test-Path $configPath)) {
        try {
            $customConfig = Get-Content $configPath -Raw | ConvertFrom-Json -AsHashtable
        }
        catch {
            Write-Verbose "Failed to load default config: $_"
        }
    }

    # Load user config (overrides)
    if (Test-Path $userConfigPath) {
        try {
            $userConfig = Get-Content $userConfigPath -Raw | ConvertFrom-Json -AsHashtable
            # Merge user config into custom config
            foreach ($section in $userConfig.Keys) {
                if ($customConfig.ContainsKey($section) -and $customConfig[$section] -is [hashtable]) {
                    foreach ($key in $userConfig[$section].Keys) {
                        $customConfig[$section][$key] = $userConfig[$section][$key]
                    }
                }
                else {
                    $customConfig[$section] = $userConfig[$section]
                }
            }
        }
        catch {
            Write-Verbose "Failed to load user config: $_"
        }
    }

    # Merge custom config into defaults
    $mergedConfig = $defaultConfig.Clone()

    foreach ($section in $customConfig.Keys) {
        if ($mergedConfig.ContainsKey($section) -and $mergedConfig[$section] -is [hashtable]) {
            foreach ($key in $customConfig[$section].Keys) {
                $mergedConfig[$section][$key] = $customConfig[$section][$key]
            }
        }
        else {
            $mergedConfig[$section] = $customConfig[$section]
        }
    }

    # Return specific key or full config
    if ($Key) {
        $parts = $Key -split '\.'
        $value = $mergedConfig

        foreach ($part in $parts) {
            if ($value -is [hashtable] -and $value.ContainsKey($part)) {
                $value = $value[$part]
            }
            else {
                return $null
            }
        }

        return $value
    }

    return $mergedConfig
}

function Set-WinMoleConfig {
    <#
    .SYNOPSIS
        Sets a WinMole configuration value.
    .DESCRIPTION
        Updates the user configuration file with the specified value.
    .PARAMETER Key
        Configuration key (dot-notation supported).
    .PARAMETER Value
        Value to set.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [string]$Key,

        [Parameter(Mandatory)]
        [object]$Value
    )

    $userConfigPath = Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'WinMole\config.json'
    $userConfigDir = Split-Path $userConfigPath -Parent

    # Ensure directory exists
    if (-not (Test-Path $userConfigDir)) {
        New-Item -Path $userConfigDir -ItemType Directory -Force | Out-Null
    }

    # Load existing config
    $config = @{}
    if (Test-Path $userConfigPath) {
        try {
            $config = Get-Content $userConfigPath -Raw | ConvertFrom-Json -AsHashtable
        }
        catch {
            $config = @{}
        }
    }

    # Set the value using dot notation
    $parts = $Key -split '\.'
    $current = $config

    for ($i = 0; $i -lt $parts.Count - 1; $i++) {
        $part = $parts[$i]
        if (-not $current.ContainsKey($part)) {
            $current[$part] = @{}
        }
        $current = $current[$part]
    }

    $current[$parts[-1]] = $Value

    # Save config
    $config | ConvertTo-Json -Depth 10 | Set-Content $userConfigPath -Force

    Write-WinMoleStatus -Message "Configuration updated: $Key = $Value" -Type Success
}

function Reset-WinMoleConfig {
    <#
    .SYNOPSIS
        Resets WinMole configuration to defaults.
    #>
    [CmdletBinding(SupportsShouldProcess)]
    param()

    $userConfigPath = Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'WinMole\config.json'

    if (Test-Path $userConfigPath) {
        if ($PSCmdlet.ShouldProcess($userConfigPath, "Remove user configuration")) {
            Remove-Item $userConfigPath -Force
            Write-WinMoleStatus -Message "Configuration reset to defaults" -Type Success
        }
    }
    else {
        Write-WinMoleStatus -Message "No custom configuration found" -Type Info
    }
}
