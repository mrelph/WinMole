function Get-WinMoleDisk {
    <#
    .SYNOPSIS
        Visualize and analyze disk usage.
    .DESCRIPTION
        Provides a tree-view disk usage visualization, finds largest files/folders,
        detects duplicates, and analyzes file types and ages.
    .PARAMETER Path
        The path to analyze. Defaults to system drive.
    .PARAMETER Depth
        Maximum depth for tree visualization.
    .PARAMETER TopN
        Number of top items to show at each level.
    .PARAMETER Mode
        Analysis mode: Tree, LargestFiles, LargestFolders, Duplicates, FileTypes, OldFiles.
    .PARAMETER OlderThan
        For OldFiles mode, find files older than this many days.
    .PARAMETER MinSize
        Minimum file size to include in analysis.
    .EXAMPLE
        Get-WinMoleDisk
        Show disk usage tree for system drive.
    .EXAMPLE
        Get-WinMoleDisk -Path C:\Users -Mode LargestFiles -TopN 20
        Find the 20 largest files in Users folder.
    .EXAMPLE
        Get-WinMoleDisk -Mode Duplicates -Path D:\Documents
        Find duplicate files in Documents.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Position = 0)]
        [string]$Path = 'C:\',

        [Parameter()]
        [ValidateRange(1, 10)]
        [int]$Depth = 3,

        [Parameter()]
        [ValidateRange(5, 100)]
        [int]$TopN = 10,

        [Parameter()]
        [ValidateSet('Tree', 'LargestFiles', 'LargestFolders', 'Duplicates', 'FileTypes', 'OldFiles', 'Summary')]
        [string]$Mode = 'Tree',

        [Parameter()]
        [int]$OlderThan = 365,

        [Parameter()]
        [long]$MinSize = 1MB
    )

    begin {
        $resolvedPath = ConvertTo-SafePath -Path $Path -Resolve -MustExist

        Write-WinMoleHeader -Title "WinMole Disk Analysis" -Style Box
        Write-Host ""
    }

    process {
        switch ($Mode) {
            'Tree' {
                Write-Tree -Path $resolvedPath -Depth $Depth -ShowSize -ShowPercent -TopN $TopN
            }

            'Summary' {
                # Drive summary
                $drives = Get-PSDrive -PSProvider FileSystem | Where-Object { $_.Used -or $_.Free }

                Write-WinMoleSection -Title "Drive Summary"
                Write-Host ""

                foreach ($drive in $drives) {
                    $total = $drive.Used + $drive.Free
                    $usedPercent = if ($total -gt 0) { [math]::Round(($drive.Used / $total) * 100) } else { 0 }

                    Write-ColorOutput "  $($drive.Name): " -ForegroundColor BrightCyan -NoNewline

                    # Progress bar
                    $barColor = switch ($usedPercent) {
                        { $_ -gt 90 } { 'Red' }
                        { $_ -gt 75 } { 'Yellow' }
                        default { 'Green' }
                    }

                    Write-ProgressBar -Percent $usedPercent -Width 25 -FillColor $barColor -NoNewline
                    Write-ColorOutput " $(Format-FileSize $drive.Used) / $(Format-FileSize $total)" -ForegroundColor White -NoNewline
                    Write-ColorOutput " ($usedPercent% used)" -ForegroundColor BrightBlack
                }

                Write-Host ""

                # Quick folder summary for analyzed path
                if (Test-Path $resolvedPath -PathType Container) {
                    Write-WinMoleSection -Title "Folder Summary: $resolvedPath"
                    Write-Host ""

                    $topFolders = Get-ChildItem -Path $resolvedPath -Directory -ErrorAction SilentlyContinue |
                        ForEach-Object {
                            $size = (Get-ChildItem -Path $_.FullName -Recurse -File -Force -ErrorAction SilentlyContinue |
                                Measure-Object -Property Length -Sum).Sum
                            [PSCustomObject]@{
                                Name = $_.Name
                                Path = $_.FullName
                                Size = $size
                            }
                        } | Sort-Object Size -Descending | Select-Object -First $TopN

                    $totalSize = ($topFolders | Measure-Object -Property Size -Sum).Sum

                    foreach ($folder in $topFolders) {
                        $percent = if ($totalSize -gt 0) { [math]::Round(($folder.Size / $totalSize) * 100) } else { 0 }
                        $bar = Write-InlineProgressBar -Percent $percent -Width 15

                        Write-ColorOutput "  $([char]0x25A0) " -ForegroundColor Yellow -NoNewline
                        Write-Host "$($folder.Name.PadRight(25)) $bar $(Format-FileSize $folder.Size)"
                    }
                }
            }

            'LargestFiles' {
                Write-WinMoleSection -Title "Largest Files in $resolvedPath"
                Write-Host ""

                Write-WinMoleStatus -Message "Scanning for largest files..." -Type Progress

                $largestFiles = Get-ChildItem -Path $resolvedPath -Recurse -File -Force -ErrorAction SilentlyContinue |
                    Where-Object { $_.Length -ge $MinSize } |
                    Sort-Object Length -Descending |
                    Select-Object -First $TopN

                if ($largestFiles) {
                    $maxSize = $largestFiles[0].Length

                    $tableData = $largestFiles | ForEach-Object {
                        $percent = [math]::Round(($_.Length / $maxSize) * 100)
                        @{
                            Size = Format-FileSize $_.Length
                            Bar = Write-InlineProgressBar -Percent $percent -Width 10
                            Name = $_.Name
                            Path = $_.DirectoryName -replace [regex]::Escape($resolvedPath), '.'
                            Modified = $_.LastWriteTime.ToString('yyyy-MM-dd')
                        }
                    }

                    Write-Table -Data $tableData -Columns @(
                        @{ Name = 'Size'; Property = 'Size'; Width = 10; Align = 'Right' }
                        @{ Name = ''; Property = 'Bar'; Width = 10 }
                        @{ Name = 'Name'; Property = 'Name'; Width = 30 }
                        @{ Name = 'Modified'; Property = 'Modified'; Width = 12 }
                    )

                    $totalSize = ($largestFiles | Measure-Object -Property Length -Sum).Sum
                    Write-Host ""
                    Write-ColorOutput "  Total: $(Format-FileSize $totalSize) in $($largestFiles.Count) files" -ForegroundColor BrightBlack
                }
                else {
                    Write-WinMoleStatus -Message "No files found matching criteria" -Type Info
                }
            }

            'LargestFolders' {
                Write-WinMoleSection -Title "Largest Folders in $resolvedPath"
                Write-Host ""

                Write-WinMoleStatus -Message "Calculating folder sizes..." -Type Progress

                $folders = Get-ChildItem -Path $resolvedPath -Directory -Force -ErrorAction SilentlyContinue |
                    ForEach-Object {
                        $size = (Get-ChildItem -Path $_.FullName -Recurse -File -Force -ErrorAction SilentlyContinue |
                            Measure-Object -Property Length -Sum).Sum
                        [PSCustomObject]@{
                            Name = $_.Name
                            Path = $_.FullName
                            Size = $size
                            FileCount = (Get-ChildItem -Path $_.FullName -Recurse -File -Force -ErrorAction SilentlyContinue).Count
                        }
                    } | Sort-Object Size -Descending | Select-Object -First $TopN

                if ($folders -and $folders[0].Size -gt 0) {
                    $maxSize = $folders[0].Size

                    foreach ($folder in $folders) {
                        $percent = [math]::Round(($folder.Size / $maxSize) * 100)
                        $bar = Write-InlineProgressBar -Percent $percent -Width 20

                        Write-ColorOutput "  $([char]0x1F4C1) " -ForegroundColor Yellow -NoNewline
                        Write-Host "$($folder.Name.PadRight(25)) $bar $(Format-FileSize $folder.Size) ($($folder.FileCount) files)"
                    }

                    $totalSize = ($folders | Measure-Object -Property Size -Sum).Sum
                    Write-Host ""
                    Write-ColorOutput "  Total: $(Format-FileSize $totalSize)" -ForegroundColor BrightBlack
                }
                else {
                    Write-WinMoleStatus -Message "No folders found or all empty" -Type Info
                }
            }

            'Duplicates' {
                Write-WinMoleSection -Title "Duplicate Files in $resolvedPath"
                Write-Host ""

                $result = Find-DuplicateFiles -Path $resolvedPath -MinSize $MinSize -Recurse

                if ($result.TotalGroups -gt 0) {
                    Write-WinMoleStatus -Message "Found $($result.TotalGroups) duplicate groups" -Type Warning
                    Write-ColorOutput "  Wasted space: $($result.TotalWastedSpaceFormatted)" -ForegroundColor Yellow
                    Write-Host ""

                    $shown = 0
                    foreach ($group in $result.Duplicates | Sort-Object WastedSpace -Descending | Select-Object -First $TopN) {
                        $shown++
                        Write-ColorOutput "  Group $shown - $($group.SizeFormatted) x $($group.Count) copies = $($group.WastedSpaceFormatted) wasted" -ForegroundColor BrightCyan

                        foreach ($file in $group.Files) {
                            $relativePath = $file -replace [regex]::Escape($resolvedPath), '.'
                            Write-ColorOutput "    $([char]0x25CF) $relativePath" -ForegroundColor BrightBlack
                        }
                        Write-Host ""
                    }
                }
                else {
                    Write-WinMoleStatus -Message "No duplicates found" -Type Success
                }
            }

            'FileTypes' {
                Write-WinMoleSection -Title "File Types in $resolvedPath"
                Write-Host ""

                Write-WinMoleStatus -Message "Analyzing file types..." -Type Progress

                $files = Get-ChildItem -Path $resolvedPath -Recurse -File -Force -ErrorAction SilentlyContinue

                $typeStats = $files | Group-Object Extension | ForEach-Object {
                    $totalSize = ($_.Group | Measure-Object -Property Length -Sum).Sum
                    [PSCustomObject]@{
                        Extension = if ($_.Name) { $_.Name.ToLower() } else { '(none)' }
                        Count = $_.Count
                        Size = $totalSize
                    }
                } | Sort-Object Size -Descending | Select-Object -First $TopN

                if ($typeStats -and $typeStats[0].Size -gt 0) {
                    $maxSize = $typeStats[0].Size

                    foreach ($type in $typeStats) {
                        $percent = [math]::Round(($type.Size / $maxSize) * 100)
                        $bar = Write-InlineProgressBar -Percent $percent -Width 15

                        $ext = $type.Extension.PadRight(10)
                        $count = "$($type.Count) files".PadRight(12)
                        $size = Format-FileSize $type.Size

                        Write-Host "  $ext $bar $count $size"
                    }

                    $totalSize = ($typeStats | Measure-Object -Property Size -Sum).Sum
                    $totalCount = ($typeStats | Measure-Object -Property Count -Sum).Sum
                    Write-Host ""
                    Write-ColorOutput "  Total: $(Format-FileSize $totalSize) in $totalCount files" -ForegroundColor BrightBlack
                }
            }

            'OldFiles' {
                Write-WinMoleSection -Title "Files Older Than $OlderThan Days in $resolvedPath"
                Write-Host ""

                $cutoffDate = (Get-Date).AddDays(-$OlderThan)

                Write-WinMoleStatus -Message "Scanning for old files..." -Type Progress

                $oldFiles = Get-ChildItem -Path $resolvedPath -Recurse -File -Force -ErrorAction SilentlyContinue |
                    Where-Object { $_.LastWriteTime -lt $cutoffDate -and $_.Length -ge $MinSize } |
                    Sort-Object Length -Descending |
                    Select-Object -First $TopN

                if ($oldFiles) {
                    foreach ($file in $oldFiles) {
                        $age = [math]::Round(((Get-Date) - $file.LastWriteTime).TotalDays)
                        $relativePath = $file.FullName -replace [regex]::Escape($resolvedPath), '.'

                        Write-ColorOutput "  $([char]0x25CF) " -ForegroundColor BrightBlack -NoNewline
                        Write-ColorOutput "$(Format-FileSize $file.Length)" -ForegroundColor White -NoNewline
                        Write-ColorOutput " - $age days old" -ForegroundColor Yellow -NoNewline
                        Write-ColorOutput " - $relativePath" -ForegroundColor BrightBlack
                    }

                    $totalSize = ($oldFiles | Measure-Object -Property Length -Sum).Sum
                    Write-Host ""
                    Write-ColorOutput "  Total: $(Format-FileSize $totalSize) in $($oldFiles.Count) old files" -ForegroundColor BrightBlack
                }
                else {
                    Write-WinMoleStatus -Message "No old files found matching criteria" -Type Success
                }
            }
        }
    }

    end {
        Write-Host ""
    }
}
