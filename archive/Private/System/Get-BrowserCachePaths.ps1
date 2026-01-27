function Get-BrowserCachePaths {
    <#
    .SYNOPSIS
        Gets browser cache paths for common browsers.
    .DESCRIPTION
        Returns information about cache, history, and temporary data locations
        for Edge, Chrome, Firefox, Brave, and other Chromium browsers.
    .PARAMETER Browser
        Filter to specific browser(s). Default is all detected browsers.
    .PARAMETER IncludeSize
        Calculate and include size information.
    .PARAMETER IncludeHistory
        Include history and cookie paths (more sensitive data).
    #>
    [CmdletBinding()]
    param(
        [Parameter()]
        [ValidateSet('All', 'Edge', 'Chrome', 'Firefox', 'Brave', 'Opera', 'Vivaldi')]
        [string]$Browser = 'All',

        [Parameter()]
        [switch]$IncludeSize,

        [Parameter()]
        [switch]$IncludeHistory
    )

    $localAppData = $env:LOCALAPPDATA
    $appData = $env:APPDATA

    # Browser definitions
    $browsers = @{
        'Edge' = @{
            Name = 'Microsoft Edge'
            BasePath = Join-Path $localAppData 'Microsoft\Edge\User Data'
            CachePaths = @(
                @{ Name = 'Cache'; Path = 'Default\Cache'; Description = 'Page cache' }
                @{ Name = 'Code Cache'; Path = 'Default\Code Cache'; Description = 'JavaScript cache' }
                @{ Name = 'GPUCache'; Path = 'Default\GPUCache'; Description = 'GPU shader cache' }
                @{ Name = 'Service Worker'; Path = 'Default\Service Worker\CacheStorage'; Description = 'Service worker cache' }
            )
            HistoryPaths = @(
                @{ Name = 'History'; Path = 'Default\History'; Description = 'Browsing history'; IsFile = $true }
                @{ Name = 'Cookies'; Path = 'Default\Network\Cookies'; Description = 'Cookies'; IsFile = $true }
                @{ Name = 'Sessions'; Path = 'Default\Sessions'; Description = 'Session data' }
            )
        }
        'Chrome' = @{
            Name = 'Google Chrome'
            BasePath = Join-Path $localAppData 'Google\Chrome\User Data'
            CachePaths = @(
                @{ Name = 'Cache'; Path = 'Default\Cache'; Description = 'Page cache' }
                @{ Name = 'Code Cache'; Path = 'Default\Code Cache'; Description = 'JavaScript cache' }
                @{ Name = 'GPUCache'; Path = 'Default\GPUCache'; Description = 'GPU shader cache' }
                @{ Name = 'Service Worker'; Path = 'Default\Service Worker\CacheStorage'; Description = 'Service worker cache' }
            )
            HistoryPaths = @(
                @{ Name = 'History'; Path = 'Default\History'; Description = 'Browsing history'; IsFile = $true }
                @{ Name = 'Cookies'; Path = 'Default\Network\Cookies'; Description = 'Cookies'; IsFile = $true }
                @{ Name = 'Sessions'; Path = 'Default\Sessions'; Description = 'Session data' }
            )
        }
        'Firefox' = @{
            Name = 'Mozilla Firefox'
            BasePath = Join-Path $appData 'Mozilla\Firefox\Profiles'
            IsProfileBased = $true
            CachePaths = @(
                @{ Name = 'Cache2'; Path = 'cache2'; Description = 'Disk cache' }
                @{ Name = 'Startup Cache'; Path = 'startupCache'; Description = 'Startup cache' }
            )
            HistoryPaths = @(
                @{ Name = 'History'; Path = 'places.sqlite'; Description = 'History and bookmarks'; IsFile = $true }
                @{ Name = 'Cookies'; Path = 'cookies.sqlite'; Description = 'Cookies'; IsFile = $true }
                @{ Name = 'Sessions'; Path = 'sessionstore-backups'; Description = 'Session backups' }
            )
        }
        'Brave' = @{
            Name = 'Brave Browser'
            BasePath = Join-Path $localAppData 'BraveSoftware\Brave-Browser\User Data'
            CachePaths = @(
                @{ Name = 'Cache'; Path = 'Default\Cache'; Description = 'Page cache' }
                @{ Name = 'Code Cache'; Path = 'Default\Code Cache'; Description = 'JavaScript cache' }
                @{ Name = 'GPUCache'; Path = 'Default\GPUCache'; Description = 'GPU shader cache' }
            )
            HistoryPaths = @(
                @{ Name = 'History'; Path = 'Default\History'; Description = 'Browsing history'; IsFile = $true }
                @{ Name = 'Cookies'; Path = 'Default\Network\Cookies'; Description = 'Cookies'; IsFile = $true }
            )
        }
        'Opera' = @{
            Name = 'Opera'
            BasePath = Join-Path $appData 'Opera Software\Opera Stable'
            CachePaths = @(
                @{ Name = 'Cache'; Path = 'Cache'; Description = 'Page cache' }
                @{ Name = 'GPUCache'; Path = 'GPUCache'; Description = 'GPU shader cache' }
            )
            HistoryPaths = @(
                @{ Name = 'History'; Path = 'History'; Description = 'Browsing history'; IsFile = $true }
            )
        }
        'Vivaldi' = @{
            Name = 'Vivaldi'
            BasePath = Join-Path $localAppData 'Vivaldi\User Data'
            CachePaths = @(
                @{ Name = 'Cache'; Path = 'Default\Cache'; Description = 'Page cache' }
                @{ Name = 'Code Cache'; Path = 'Default\Code Cache'; Description = 'JavaScript cache' }
            )
            HistoryPaths = @(
                @{ Name = 'History'; Path = 'Default\History'; Description = 'Browsing history'; IsFile = $true }
            )
        }
    }

    $results = @()

    foreach ($browserKey in $browsers.Keys) {
        if ($Browser -ne 'All' -and $Browser -ne $browserKey) {
            continue
        }

        $browserInfo = $browsers[$browserKey]
        $basePath = $browserInfo.BasePath

        # Check if browser is installed
        if (-not (Test-Path $basePath)) {
            continue
        }

        # Handle profile-based browsers (Firefox)
        $profilePaths = @($basePath)
        if ($browserInfo.IsProfileBased) {
            $profiles = Get-ChildItem -Path $basePath -Directory -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -match '\.default' -or $_.Name -match '^[a-z0-9]+\.' }
            $profilePaths = $profiles.FullName
        }

        foreach ($profilePath in $profilePaths) {
            $actualBasePath = if ($browserInfo.IsProfileBased) { $profilePath } else { $basePath }

            # Process cache paths
            foreach ($cachePath in $browserInfo.CachePaths) {
                $fullPath = Join-Path $actualBasePath $cachePath.Path

                $entry = @{
                    Browser = $browserInfo.Name
                    BrowserKey = $browserKey
                    Name = $cachePath.Name
                    Path = $fullPath
                    Description = $cachePath.Description
                    Type = 'Cache'
                    Exists = Test-Path $fullPath
                    IsFile = $cachePath.IsFile -eq $true
                }

                if ($IncludeSize -and $entry.Exists) {
                    try {
                        if ($entry.IsFile) {
                            $item = Get-Item $fullPath -ErrorAction SilentlyContinue
                            $entry.Size = if ($item) { $item.Length } else { 0 }
                            $entry.FileCount = 1
                        } else {
                            $items = Get-ChildItem -Path $fullPath -Recurse -File -Force -ErrorAction SilentlyContinue
                            $entry.Size = ($items | Measure-Object -Property Length -Sum).Sum
                            $entry.FileCount = $items.Count
                        }
                        $entry.SizeFormatted = Format-FileSize $entry.Size
                    }
                    catch {
                        $entry.Size = 0
                        $entry.FileCount = 0
                        $entry.AccessDenied = $true
                    }
                }

                $results += [PSCustomObject]$entry
            }

            # Process history paths if requested
            if ($IncludeHistory) {
                foreach ($histPath in $browserInfo.HistoryPaths) {
                    $fullPath = Join-Path $actualBasePath $histPath.Path

                    $entry = @{
                        Browser = $browserInfo.Name
                        BrowserKey = $browserKey
                        Name = $histPath.Name
                        Path = $fullPath
                        Description = $histPath.Description
                        Type = 'History'
                        Exists = Test-Path $fullPath
                        IsFile = $histPath.IsFile -eq $true
                        Sensitive = $true
                    }

                    if ($IncludeSize -and $entry.Exists) {
                        try {
                            if ($entry.IsFile) {
                                $item = Get-Item $fullPath -ErrorAction SilentlyContinue
                                $entry.Size = if ($item) { $item.Length } else { 0 }
                                $entry.FileCount = 1
                            } else {
                                $items = Get-ChildItem -Path $fullPath -Recurse -File -Force -ErrorAction SilentlyContinue
                                $entry.Size = ($items | Measure-Object -Property Length -Sum).Sum
                                $entry.FileCount = $items.Count
                            }
                            $entry.SizeFormatted = Format-FileSize $entry.Size
                        }
                        catch {
                            $entry.Size = 0
                            $entry.AccessDenied = $true
                        }
                    }

                    $results += [PSCustomObject]$entry
                }
            }
        }
    }

    return $results
}

function Get-BrowserCacheSummary {
    <#
    .SYNOPSIS
        Gets a summary of browser cache sizes.
    #>
    [CmdletBinding()]
    param()

    $caches = Get-BrowserCachePaths -IncludeSize | Where-Object { $_.Exists -and $_.Type -eq 'Cache' }

    $summary = $caches | Group-Object Browser | ForEach-Object {
        $totalSize = ($_.Group | Measure-Object -Property Size -Sum).Sum
        [PSCustomObject]@{
            Browser = $_.Name
            CacheCount = $_.Count
            TotalSize = $totalSize
            TotalSizeFormatted = Format-FileSize $totalSize
        }
    }

    $grandTotal = ($caches | Measure-Object -Property Size -Sum).Sum

    return @{
        Browsers = $summary
        TotalSize = $grandTotal
        TotalSizeFormatted = Format-FileSize $grandTotal
    }
}
