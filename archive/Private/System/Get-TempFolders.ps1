function Get-TempFolders {
    <#
    .SYNOPSIS
        Gets a list of temporary folders to clean.
    .DESCRIPTION
        Returns information about various temporary folders on the system
        including their paths, descriptions, and whether admin rights are needed.
    .PARAMETER Category
        Filter by category (User, System, Browser, Windows).
    .PARAMETER IncludeSize
        Calculate and include the size of each folder.
    #>
    [CmdletBinding()]
    param(
        [Parameter()]
        [ValidateSet('All', 'User', 'System', 'Browser', 'Windows', 'Cache')]
        [string]$Category = 'All',

        [Parameter()]
        [switch]$IncludeSize
    )

    $folders = @(
        # User temp folders
        @{
            Name = 'User Temp'
            Path = $env:TEMP
            Description = 'User temporary files'
            Category = 'User'
            RequiresAdmin = $false
            Priority = 'High'
        },
        @{
            Name = 'User Temp (Alt)'
            Path = Join-Path $env:LOCALAPPDATA 'Temp'
            Description = 'User local app data temp'
            Category = 'User'
            RequiresAdmin = $false
            Priority = 'High'
        },

        # System temp folders
        @{
            Name = 'Windows Temp'
            Path = 'C:\Windows\Temp'
            Description = 'System temporary files'
            Category = 'System'
            RequiresAdmin = $true
            Priority = 'High'
        },

        # Windows Update
        @{
            Name = 'Windows Update Cache'
            Path = 'C:\Windows\SoftwareDistribution\Download'
            Description = 'Windows Update downloaded files'
            Category = 'Windows'
            RequiresAdmin = $true
            Priority = 'Medium'
        },

        # Prefetch
        @{
            Name = 'Prefetch'
            Path = 'C:\Windows\Prefetch'
            Description = 'Application prefetch cache'
            Category = 'Windows'
            RequiresAdmin = $true
            Priority = 'Low'
        },

        # Thumbnail cache
        @{
            Name = 'Thumbnail Cache'
            Path = Join-Path $env:LOCALAPPDATA 'Microsoft\Windows\Explorer'
            Description = 'Windows Explorer thumbnails'
            Category = 'Cache'
            RequiresAdmin = $false
            Priority = 'Medium'
            Pattern = 'thumbcache_*.db'
        },

        # Windows Error Reporting
        @{
            Name = 'Error Reports (User)'
            Path = Join-Path $env:LOCALAPPDATA 'Microsoft\Windows\WER'
            Description = 'Windows Error Reporting (user)'
            Category = 'Windows'
            RequiresAdmin = $false
            Priority = 'Medium'
        },
        @{
            Name = 'Error Reports (System)'
            Path = 'C:\ProgramData\Microsoft\Windows\WER'
            Description = 'Windows Error Reporting (system)'
            Category = 'Windows'
            RequiresAdmin = $true
            Priority = 'Medium'
        },

        # Delivery Optimization
        @{
            Name = 'Delivery Optimization'
            Path = 'C:\Windows\ServiceProfiles\NetworkService\AppData\Local\Microsoft\Windows\DeliveryOptimization'
            Description = 'Windows Update delivery optimization cache'
            Category = 'Windows'
            RequiresAdmin = $true
            Priority = 'Medium'
        },

        # Windows.old
        @{
            Name = 'Previous Windows Installation'
            Path = 'C:\Windows.old'
            Description = 'Previous Windows installation (after upgrade)'
            Category = 'Windows'
            RequiresAdmin = $true
            Priority = 'Low'
            Dangerous = $true
        },

        # Recent files
        @{
            Name = 'Recent Items'
            Path = Join-Path $env:APPDATA 'Microsoft\Windows\Recent'
            Description = 'Recent files shortcuts'
            Category = 'User'
            RequiresAdmin = $false
            Priority = 'Low'
        },

        # Installer cache
        @{
            Name = 'Windows Installer Cache'
            Path = 'C:\Windows\Installer\$PatchCache$'
            Description = 'Windows Installer patch cache'
            Category = 'Windows'
            RequiresAdmin = $true
            Priority = 'Low'
            Dangerous = $true
        },

        # Memory dumps
        @{
            Name = 'Memory Dumps'
            Path = 'C:\Windows\Minidump'
            Description = 'System crash minidumps'
            Category = 'System'
            RequiresAdmin = $true
            Priority = 'Medium'
        },
        @{
            Name = 'Memory Dump (Full)'
            Path = 'C:\Windows\MEMORY.DMP'
            Description = 'Full system memory dump'
            Category = 'System'
            RequiresAdmin = $true
            Priority = 'High'
            IsFile = $true
        },

        # Font cache
        @{
            Name = 'Font Cache'
            Path = 'C:\Windows\ServiceProfiles\LocalService\AppData\Local\FontCache'
            Description = 'Windows font cache'
            Category = 'Cache'
            RequiresAdmin = $true
            Priority = 'Low'
        },

        # Icon cache
        @{
            Name = 'Icon Cache'
            Path = Join-Path $env:LOCALAPPDATA 'Microsoft\Windows\Explorer'
            Description = 'Windows icon cache'
            Category = 'Cache'
            RequiresAdmin = $false
            Priority = 'Low'
            Pattern = 'iconcache_*.db'
        }
    )

    # Filter by category
    if ($Category -ne 'All') {
        $folders = $folders | Where-Object { $_.Category -eq $Category }
    }

    # Calculate sizes if requested
    if ($IncludeSize) {
        foreach ($folder in $folders) {
            if (-not (Test-Path $folder.Path)) {
                $folder.Size = 0
                $folder.Exists = $false
                continue
            }

            $folder.Exists = $true

            try {
                if ($folder.IsFile) {
                    $item = Get-Item $folder.Path -ErrorAction SilentlyContinue
                    $folder.Size = if ($item) { $item.Length } else { 0 }
                    $folder.FileCount = 1
                }
                elseif ($folder.Pattern) {
                    $items = Get-ChildItem -Path $folder.Path -Filter $folder.Pattern -ErrorAction SilentlyContinue
                    $folder.Size = ($items | Measure-Object -Property Length -Sum).Sum
                    $folder.FileCount = $items.Count
                }
                else {
                    $items = Get-ChildItem -Path $folder.Path -Recurse -File -Force -ErrorAction SilentlyContinue
                    $folder.Size = ($items | Measure-Object -Property Length -Sum).Sum
                    $folder.FileCount = $items.Count
                }
            }
            catch {
                $folder.Size = 0
                $folder.FileCount = 0
                $folder.AccessDenied = $true
            }

            $folder.SizeFormatted = Format-FileSize $folder.Size
        }
    }

    return $folders
}

function Get-TempFolderSummary {
    <#
    .SYNOPSIS
        Gets a summary of temp folder sizes by category.
    #>
    [CmdletBinding()]
    param()

    $folders = Get-TempFolders -IncludeSize

    $summary = $folders | Group-Object Category | ForEach-Object {
        $totalSize = ($_.Group | Measure-Object -Property Size -Sum).Sum
        [PSCustomObject]@{
            Category = $_.Name
            FolderCount = $_.Count
            TotalSize = $totalSize
            TotalSizeFormatted = Format-FileSize $totalSize
            Folders = $_.Group
        }
    }

    $grandTotal = ($folders | Measure-Object -Property Size -Sum).Sum

    return @{
        Categories = $summary
        TotalSize = $grandTotal
        TotalSizeFormatted = Format-FileSize $grandTotal
        FolderCount = $folders.Count
    }
}
