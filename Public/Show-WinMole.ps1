function Show-WinMole {
    <#
    .SYNOPSIS
        Interactive main menu for WinMole.
    .DESCRIPTION
        Provides an interactive menu interface to access all WinMole features.
    .PARAMETER Quick
        Skip the menu and run a quick system scan.
    .EXAMPLE
        Show-WinMole
        Launch the interactive menu.
    .EXAMPLE
        Show-WinMole -Quick
        Run a quick system overview.
    #>
    [CmdletBinding()]
    param(
        [Parameter()]
        [switch]$Quick,

        [Parameter()]
        [switch]$Simple
    )

    if ($Quick) {
        # Quick mode - show system status
        Write-WinMoleHeader -Title "WinMole Quick Scan" -Style Box
        Write-Host ""

        $health = Get-HealthScore -Detailed
        $healthColor = Get-HealthColor -Score $health.Score

        Write-ColorOutput "  Health Score: " -ForegroundColor White -NoNewline
        Write-ColorOutput "$($health.Score)/100" -ForegroundColor $healthColor -Style Bold -NoNewline
        Write-ColorOutput " ($($health.Status))" -ForegroundColor BrightBlack
        Write-Host ""

        # Quick summary of cleanable space
        Write-WinMoleStatus -Message "Checking for cleanable files..." -Type Progress

        $tempFolders = Get-TempFolders -IncludeSize | Where-Object { $_.Exists }
        $tempSize = ($tempFolders | Measure-Object -Property Size -Sum).Sum

        $browserCaches = Get-BrowserCachePaths -IncludeSize | Where-Object { $_.Exists -and $_.Type -eq 'Cache' }
        $browserSize = ($browserCaches | Measure-Object -Property Size -Sum).Sum

        $totalCleanable = $tempSize + $browserSize

        Write-Host ""
        Write-ColorOutput "  Cleanable space found: " -ForegroundColor White -NoNewline
        Write-ColorOutput (Format-FileSize $totalCleanable) -ForegroundColor Yellow -Style Bold
        Write-Host ""
        Write-ColorOutput "    Temp files: $(Format-FileSize $tempSize)" -ForegroundColor BrightBlack
        Write-ColorOutput "    Browser cache: $(Format-FileSize $browserSize)" -ForegroundColor BrightBlack

        if ($health.Recommendations.Count -gt 0) {
            Write-Host ""
            Write-WinMoleSection -Title "Recommendations"
            foreach ($rec in $health.Recommendations) {
                Write-WinMoleStatus -Message $rec -Type Warning
            }
        }

        Write-Host ""
        Write-ColorOutput "  Run 'Show-WinMole' for full interactive menu" -ForegroundColor BrightBlack
        Write-ColorOutput "  Run 'Invoke-WinMoleClean -WhatIf' to preview cleanup" -ForegroundColor BrightBlack
        Write-Host ""

        return
    }

    # Interactive menu mode
    while ($true) {
        $menuOptions = @(
            @{
                Name = "System Cleanup"
                Description = "Clean temp files, browser caches, and system junk"
                Action = {
                    Clear-Host
                    Invoke-WinMoleClean -WhatIf
                    Write-Host ""
                    $confirm = Show-Confirmation -Message "Proceed with cleanup?"
                    if ($confirm) {
                        Invoke-WinMoleClean -Force
                    }
                    Read-Host "Press Enter to continue"
                }
            },
            @{
                Name = "Disk Analysis"
                Description = "Analyze disk usage and find large files"
                Action = {
                    Clear-Host
                    $mode = Show-Menu -Title "Disk Analysis Mode" -Options @(
                        @{ Name = "Tree View"; Value = 'Tree' }
                        @{ Name = "Largest Files"; Value = 'LargestFiles' }
                        @{ Name = "Largest Folders"; Value = 'LargestFolders' }
                        @{ Name = "File Types"; Value = 'FileTypes' }
                        @{ Name = "Find Duplicates"; Value = 'Duplicates' }
                        @{ Name = "Old Files"; Value = 'OldFiles' }
                    )

                    if ($mode) {
                        Clear-Host
                        $path = Read-Host "Enter path to analyze (or press Enter for C:\Users)"
                        if ([string]::IsNullOrWhiteSpace($path)) { $path = "C:\Users" }
                        Get-WinMoleDisk -Path $path -Mode $mode
                    }
                    Read-Host "Press Enter to continue"
                }
            },
            @{
                Name = "System Status"
                Description = "View real-time system health and performance"
                Action = {
                    Clear-Host
                    Get-WinMoleStatus
                    Write-Host ""
                    $live = Show-Confirmation -Message "Enable live monitoring?"
                    if ($live) {
                        Get-WinMoleStatus -Live
                    }
                    Read-Host "Press Enter to continue"
                }
            },
            @{
                Name = "Developer Cleanup"
                Description = "Remove build artifacts (node_modules, bin, obj, etc.)"
                Action = {
                    Clear-Host
                    $path = Read-Host "Enter development folder path (or press Enter for current directory)"
                    if ([string]::IsNullOrWhiteSpace($path)) { $path = "." }

                    Clear-WinMoleDevArtifacts -Path $path -WhatIf

                    Write-Host ""
                    $confirm = Show-Confirmation -Message "Proceed with cleanup?"
                    if ($confirm) {
                        Clear-WinMoleDevArtifacts -Path $path -Force
                    }
                    Read-Host "Press Enter to continue"
                }
            },
            @{
                Name = "Package Manager (winget)"
                Description = "Manage installed applications with winget"
                Action = {
                    Clear-Host
                    $action = Show-Menu -Title "Package Manager" -Options @(
                        @{ Name = "List Installed"; Value = 'list' }
                        @{ Name = "Check for Updates"; Value = 'audit' }
                        @{ Name = "Update All"; Value = 'update' }
                        @{ Name = "Search Packages"; Value = 'search' }
                        @{ Name = "Export Package List"; Value = 'export' }
                    )

                    if ($action) {
                        Clear-Host
                        switch ($action) {
                            'list' { Invoke-WinMoleWinget list }
                            'audit' { Invoke-WinMoleWinget audit }
                            'update' {
                                Invoke-WinMoleWinget audit
                                Write-Host ""
                                $confirm = Show-Confirmation -Message "Update all packages?"
                                if ($confirm) {
                                    Invoke-WinMoleWinget update -All
                                }
                            }
                            'search' {
                                $query = Read-Host "Enter search query"
                                if ($query) {
                                    Invoke-WinMoleWinget search -Query $query
                                }
                            }
                            'export' { Invoke-WinMoleWinget export }
                        }
                    }
                    Read-Host "Press Enter to continue"
                }
            },
            @{
                Name = "Registry Cleaner"
                Description = "Scan and clean orphaned registry entries"
                Action = {
                    Clear-Host
                    $result = Invoke-WinMoleRegistry -Mode Scan

                    if ($result.TotalIssues -gt 0) {
                        Write-Host ""
                        $confirm = Show-Confirmation -Message "Clean $($result.TotalIssues) registry issues?"
                        if ($confirm) {
                            Invoke-WinMoleRegistry -Mode Clean
                        }
                    }
                    Read-Host "Press Enter to continue"
                }
            },
            @{
                Name = "Startup Optimizer"
                Description = "Manage startup programs and boot performance"
                Action = {
                    Clear-Host
                    $action = Show-Menu -Title "Startup Optimizer" -Options @(
                        @{ Name = "List Startup Items"; Value = 'list' }
                        @{ Name = "Boot Impact Analysis"; Value = 'analyze' }
                        @{ Name = "Disable Item"; Value = 'disable' }
                        @{ Name = "Enable Item"; Value = 'enable' }
                    )

                    if ($action) {
                        Clear-Host
                        switch ($action) {
                            'list' { Optimize-WinMoleStartup -Action List -ShowImpact }
                            'analyze' { Optimize-WinMoleStartup -Action Analyze }
                            'disable' {
                                Optimize-WinMoleStartup -Action List
                                Write-Host ""
                                $name = Read-Host "Enter name of item to disable"
                                if ($name) {
                                    Optimize-WinMoleStartup -Action Disable -Name $name
                                }
                            }
                            'enable' {
                                Optimize-WinMoleStartup -Action List
                                Write-Host ""
                                $name = Read-Host "Enter name of item to enable"
                                if ($name) {
                                    Optimize-WinMoleStartup -Action Enable -Name $name
                                }
                            }
                        }
                    }
                    Read-Host "Press Enter to continue"
                }
            },
            @{
                Name = "Quick Scan"
                Description = "Run a quick system health check"
                Action = {
                    Clear-Host
                    Show-WinMole -Quick
                    Read-Host "Press Enter to continue"
                }
            },
            @{
                Name = "Exit"
                Description = "Exit WinMole"
                Action = { }
            }
        )

        $selection = Show-Menu -Title "WinMole - Windows System Optimization" -Options $menuOptions -Simple:$Simple

        if ($null -eq $selection -or $selection -eq 'Exit') {
            Clear-Host
            Write-ColorOutput "Thanks for using WinMole!" -ForegroundColor Cyan
            Write-Host ""
            break
        }
    }
}

# Set alias
Set-Alias -Name 'winmole' -Value 'Show-WinMole' -Scope Global
