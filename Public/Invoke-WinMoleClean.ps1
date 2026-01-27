function Invoke-WinMoleClean {
    <#
    .SYNOPSIS
        Clean temporary files and caches to reclaim disk space.
    .DESCRIPTION
        Scans and removes temporary files from various locations including
        Windows temp folders, browser caches, Windows Update cache, and more.
    .PARAMETER Category
        Target specific categories: User, System, Browser, Windows, Cache, All.
    .PARAMETER Exclude
        Paths or patterns to exclude from cleaning.
    .PARAMETER Force
        Skip confirmation prompts.
    .PARAMETER WhatIf
        Preview what would be cleaned without actually deleting.
    .EXAMPLE
        Invoke-WinMoleClean -WhatIf
        Preview cleanup without deleting anything.
    .EXAMPLE
        Invoke-WinMoleClean -Category Browser -Force
        Clean all browser caches without confirmation.
    .EXAMPLE
        Invoke-WinMoleClean -Exclude '*Chrome*'
        Clean everything except Chrome-related paths.
    #>
    [CmdletBinding(SupportsShouldProcess, ConfirmImpact = 'High')]
    param(
        [Parameter()]
        [ValidateSet('All', 'User', 'System', 'Browser', 'Windows', 'Cache')]
        [string[]]$Category = @('User', 'Browser', 'Cache'),

        [Parameter()]
        [string[]]$Exclude,

        [Parameter()]
        [switch]$Force,

        [Parameter()]
        [Alias('DryRun')]
        [switch]$Preview
    )

    begin {
        # Handle WhatIf preference
        $isPreview = $Preview -or $WhatIfPreference

        Write-WinMoleHeader -Title "WinMole System Cleanup" -Style Box

        if ($isPreview) {
            Write-WinMoleStatus -Message "Running in PREVIEW mode - no files will be deleted" -Type Warning
            Write-Host ""
        }

        $totalSpaceSaved = 0
        $totalFilesDeleted = 0
        $totalErrors = 0

        # Check admin for system categories
        $needsAdmin = $Category -contains 'All' -or $Category -contains 'System' -or $Category -contains 'Windows'
        if ($needsAdmin -and -not (Test-AdminRights -Quiet)) {
            Write-AdminWarning -Operation "Cleaning system folders"
        }
    }

    process {
        # Get temp folders based on category
        $targetCategories = if ($Category -contains 'All') { @('User', 'System', 'Browser', 'Windows', 'Cache') } else { $Category }

        foreach ($cat in $targetCategories) {
            Write-WinMoleSection -Title "Cleaning: $cat"

            if ($cat -eq 'Browser') {
                # Handle browser caches separately
                $browserCaches = Get-BrowserCachePaths -IncludeSize | Where-Object { $_.Exists -and $_.Type -eq 'Cache' }

                if ($Exclude) {
                    $browserCaches = $browserCaches | Where-Object {
                        $path = $_.Path
                        -not ($Exclude | Where-Object { $path -like $_ })
                    }
                }

                $groupedCaches = $browserCaches | Group-Object Browser

                foreach ($browserGroup in $groupedCaches) {
                    $browserTotal = ($browserGroup.Group | Measure-Object -Property Size -Sum).Sum
                    Write-Host ""
                    Write-ColorOutput "  $($browserGroup.Name)" -ForegroundColor BrightCyan -NoNewline
                    Write-ColorOutput " ($(Format-FileSize $browserTotal))" -ForegroundColor BrightBlack

                    foreach ($cache in $browserGroup.Group) {
                        $icon = if ($cache.Size -gt 100MB) { [char]0x26A0 } else { [char]0x25CF }
                        $sizeStr = Format-FileSize $cache.Size

                        Write-ColorOutput "    $icon " -ForegroundColor $(if ($cache.Size -gt 100MB) { 'Yellow' } else { 'BrightBlack' }) -NoNewline
                        Write-ColorOutput "$($cache.Name)" -ForegroundColor White -NoNewline
                        Write-ColorOutput " - $sizeStr" -ForegroundColor BrightBlack

                        if (-not $isPreview) {
                            if ($PSCmdlet.ShouldProcess($cache.Path, "Delete cache")) {
                                try {
                                    $items = Get-SafeDeletePath -Path $cache.Path
                                    foreach ($item in $items) {
                                        Remove-Item -LiteralPath $item.FullName -Recurse -Force -ErrorAction SilentlyContinue
                                        $totalFilesDeleted++
                                    }
                                    $totalSpaceSaved += $cache.Size
                                }
                                catch {
                                    $totalErrors++
                                    Write-Verbose "Error cleaning $($cache.Path): $_"
                                }
                            }
                        }
                        else {
                            $totalSpaceSaved += $cache.Size
                            $totalFilesDeleted += $cache.FileCount
                        }
                    }
                }
            }
            else {
                # Handle temp folders
                $folders = Get-TempFolders -Category $cat -IncludeSize | Where-Object { $_.Exists }

                if ($Exclude) {
                    $folders = $folders | Where-Object {
                        $path = $_.Path
                        -not ($Exclude | Where-Object { $path -like $_ })
                    }
                }

                foreach ($folder in $folders) {
                    # Skip folders that need admin if we don't have it
                    if ($folder.RequiresAdmin -and -not (Test-AdminRights -Quiet)) {
                        Write-ColorOutput "    [char]0x274C " -ForegroundColor Red -NoNewline
                        Write-ColorOutput "$($folder.Name)" -ForegroundColor BrightBlack -NoNewline
                        Write-ColorOutput " - Requires admin" -ForegroundColor Red
                        continue
                    }

                    # Skip dangerous folders unless force is specified
                    if ($folder.Dangerous -and -not $Force) {
                        Write-ColorOutput "    [char]0x26A0 " -ForegroundColor Yellow -NoNewline
                        Write-ColorOutput "$($folder.Name)" -ForegroundColor Yellow -NoNewline
                        Write-ColorOutput " - Skipped (use -Force)" -ForegroundColor BrightBlack
                        continue
                    }

                    $icon = if ($folder.Size -gt 100MB) { [char]0x26A0 } else { [char]0x25CF }
                    $sizeStr = $folder.SizeFormatted ?? (Format-FileSize $folder.Size)

                    Write-ColorOutput "    $icon " -ForegroundColor $(if ($folder.Size -gt 100MB) { 'Yellow' } else { 'BrightBlack' }) -NoNewline
                    Write-ColorOutput "$($folder.Name)" -ForegroundColor White -NoNewline
                    Write-ColorOutput " - $sizeStr" -ForegroundColor BrightBlack

                    if (-not $isPreview) {
                        if ($PSCmdlet.ShouldProcess($folder.Path, "Clean folder")) {
                            try {
                                if ($folder.IsFile) {
                                    Remove-Item -LiteralPath $folder.Path -Force -ErrorAction SilentlyContinue
                                    $totalFilesDeleted++
                                    $totalSpaceSaved += $folder.Size
                                }
                                elseif ($folder.Pattern) {
                                    $items = Get-ChildItem -Path $folder.Path -Filter $folder.Pattern -ErrorAction SilentlyContinue
                                    foreach ($item in $items) {
                                        Remove-Item -LiteralPath $item.FullName -Force -ErrorAction SilentlyContinue
                                        $totalFilesDeleted++
                                    }
                                    $totalSpaceSaved += $folder.Size
                                }
                                else {
                                    $items = Get-SafeDeletePath -Path $folder.Path
                                    foreach ($item in $items) {
                                        Remove-Item -LiteralPath $item.FullName -Recurse -Force -ErrorAction SilentlyContinue
                                        $totalFilesDeleted++
                                    }
                                    $totalSpaceSaved += $folder.Size
                                }
                            }
                            catch {
                                $totalErrors++
                                Write-Verbose "Error cleaning $($folder.Path): $_"
                            }
                        }
                    }
                    else {
                        $totalSpaceSaved += $folder.Size
                        $totalFilesDeleted += $folder.FileCount ?? 1
                    }
                }
            }
        }

        # Clear DNS cache if System or Windows category
        if (($Category -contains 'All' -or $Category -contains 'System' -or $Category -contains 'Windows') -and (Test-AdminRights -Quiet)) {
            Write-WinMoleSection -Title "Clearing DNS Cache"
            if (-not $isPreview) {
                if ($PSCmdlet.ShouldProcess("DNS Cache", "Flush")) {
                    try {
                        Clear-DnsClientCache -ErrorAction SilentlyContinue
                        Write-WinMoleStatus -Message "DNS cache cleared" -Type Success
                    }
                    catch {
                        Write-WinMoleStatus -Message "Failed to clear DNS cache" -Type Warning
                    }
                }
            }
            else {
                Write-ColorOutput "    Would clear DNS cache" -ForegroundColor BrightBlack
            }
        }

        # Empty Recycle Bin if User category
        if ($Category -contains 'All' -or $Category -contains 'User') {
            Write-WinMoleSection -Title "Recycle Bin"

            try {
                $shell = New-Object -ComObject Shell.Application
                $recycleBin = $shell.NameSpace(0x0a)
                $recycleBinItems = $recycleBin.Items()
                $recycleBinCount = $recycleBinItems.Count

                if ($recycleBinCount -gt 0) {
                    # Estimate size (can't easily get actual size)
                    Write-ColorOutput "    $([char]0x1F5D1) Recycle Bin" -ForegroundColor White -NoNewline
                    Write-ColorOutput " - $recycleBinCount items" -ForegroundColor BrightBlack

                    if (-not $isPreview) {
                        if ($PSCmdlet.ShouldProcess("Recycle Bin", "Empty ($recycleBinCount items)")) {
                            Clear-RecycleBin -Force -ErrorAction SilentlyContinue
                            $totalFilesDeleted += $recycleBinCount
                            Write-WinMoleStatus -Message "Recycle Bin emptied" -Type Success
                        }
                    }
                }
                else {
                    Write-ColorOutput "    Recycle Bin is empty" -ForegroundColor BrightBlack
                }

                [System.Runtime.InteropServices.Marshal]::ReleaseComObject($shell) | Out-Null
            }
            catch {
                Write-Verbose "Error accessing Recycle Bin: $_"
            }
        }
    }

    end {
        # Summary
        Write-Host ""
        Write-WinMoleHeader -Title "Cleanup Summary" -Style Line

        $summaryColor = if ($isPreview) { 'Yellow' } else { 'Green' }
        $actionWord = if ($isPreview) { 'Would free' } else { 'Freed' }

        Write-Host ""
        Write-ColorOutput "  $actionWord: " -ForegroundColor White -NoNewline
        Write-ColorOutput (Format-FileSize $totalSpaceSaved) -ForegroundColor $summaryColor -Style Bold

        Write-ColorOutput "  Items processed: " -ForegroundColor White -NoNewline
        Write-ColorOutput $totalFilesDeleted -ForegroundColor $summaryColor

        if ($totalErrors -gt 0) {
            Write-ColorOutput "  Errors: " -ForegroundColor White -NoNewline
            Write-ColorOutput $totalErrors -ForegroundColor Red
        }

        Write-Host ""

        if ($isPreview) {
            Write-WinMoleStatus -Message "Run without -WhatIf/-Preview to perform cleanup" -Type Info
        }
        else {
            Write-WinMoleStatus -Message "Cleanup complete!" -Type Success
        }

        # Return summary object
        return [PSCustomObject]@{
            SpaceSaved = $totalSpaceSaved
            SpaceSavedFormatted = Format-FileSize $totalSpaceSaved
            FilesDeleted = $totalFilesDeleted
            Errors = $totalErrors
            WasPreview = $isPreview
        }
    }
}
