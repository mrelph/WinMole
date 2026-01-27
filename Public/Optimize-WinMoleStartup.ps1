function Optimize-WinMoleStartup {
    <#
    .SYNOPSIS
        Manage and optimize startup applications.
    .DESCRIPTION
        Lists, enables, disables, and optimizes startup items from Registry,
        Startup folders, Scheduled Tasks, and Services.
    .PARAMETER Action
        Operation: List (default), Disable, Enable, Remove, Analyze.
    .PARAMETER Name
        Name or pattern of startup item to act on.
    .PARAMETER Source
        Filter by source: Registry, StartupFolder, ScheduledTask, Service, All.
    .PARAMETER ShowImpact
        Show estimated boot impact.
    .PARAMETER Force
        Skip confirmation prompts.
    .EXAMPLE
        Optimize-WinMoleStartup
        List all startup items.
    .EXAMPLE
        Optimize-WinMoleStartup -Action Disable -Name "Discord"
        Disable Discord startup.
    .EXAMPLE
        Optimize-WinMoleStartup -Action Analyze
        Show boot impact analysis.
    #>
    [CmdletBinding(SupportsShouldProcess)]
    param(
        [Parameter()]
        [ValidateSet('List', 'Disable', 'Enable', 'Remove', 'Analyze')]
        [string]$Action = 'List',

        [Parameter(Position = 0)]
        [string]$Name,

        [Parameter()]
        [ValidateSet('All', 'Registry', 'StartupFolder', 'ScheduledTask', 'Service')]
        [string]$Source = 'All',

        [Parameter()]
        [switch]$ShowImpact,

        [Parameter()]
        [switch]$Force,

        [Parameter()]
        [switch]$IncludeServices
    )

    begin {
        Write-WinMoleHeader -Title "WinMole Startup Optimizer" -Style Box
        Write-Host ""

        # Get startup items
        $sourceFilter = if ($Source -eq 'All') { 'All' } else { $Source }
        if (-not $IncludeServices -and $sourceFilter -eq 'All') {
            $sourceFilter = 'All'  # We'll filter services later
        }

        $startupItems = Get-StartupItems -Source $sourceFilter -IncludeDisabled

        if (-not $IncludeServices) {
            $startupItems = $startupItems | Where-Object { $_.Source -ne 'Service' }
        }

        # Add impact estimation
        if ($ShowImpact -or $Action -eq 'Analyze') {
            foreach ($item in $startupItems) {
                $impact = Get-StartupImpact -StartupItem $item
                $item | Add-Member -NotePropertyName 'Impact' -NotePropertyValue $impact.Impact -Force
                $item | Add-Member -NotePropertyName 'EstimatedSeconds' -NotePropertyValue $impact.EstimatedSeconds -Force
            }
        }
    }

    process {
        switch ($Action) {
            'List' {
                $grouped = $startupItems | Group-Object Source

                foreach ($group in $grouped | Sort-Object Name) {
                    Write-WinMoleSection -Title "$($group.Name) ($($group.Count) items)"
                    Write-Host ""

                    $items = $group.Group | Sort-Object Enabled, Name -Descending

                    foreach ($item in $items) {
                        $statusIcon = if ($item.Enabled) {
                            Get-ColoredString -Text [char]0x2713 -ForegroundColor Green
                        } else {
                            Get-ColoredString -Text [char]0x2717 -ForegroundColor Red
                        }

                        $signedIcon = if ($item.IsSigned) {
                            Get-ColoredString -Text [char]0x1F512 -ForegroundColor Green
                        } else {
                            Get-ColoredString -Text [char]0x26A0 -ForegroundColor Yellow
                        }

                        $categoryColor = switch ($item.Category) {
                            'Microsoft' { 'Cyan' }
                            'Third-party' { 'White' }
                            default { 'Yellow' }
                        }

                        $nameStr = $item.Name
                        if ($nameStr.Length -gt 25) { $nameStr = $nameStr.Substring(0, 22) + '...' }

                        Write-Host "  $statusIcon " -NoNewline
                        Write-ColorOutput $nameStr.PadRight(25) -ForegroundColor $categoryColor -NoNewline
                        Write-ColorOutput $item.Category.PadRight(12) -ForegroundColor BrightBlack -NoNewline

                        if ($ShowImpact -and $item.EstimatedSeconds) {
                            $impactColor = switch ($item.Impact) {
                                'High' { 'Red' }
                                'Medium' { 'Yellow' }
                                'Low' { 'Green' }
                                default { 'BrightBlack' }
                            }
                            $impactBar = Write-InlineProgressBar -Percent ([math]::Min(100, $item.EstimatedSeconds * 20)) -Width 5 -Color $impactColor
                            Write-Host "$impactBar $($item.Impact.PadRight(6)) $($item.EstimatedSeconds)s"
                        } else {
                            Write-Host ""
                        }
                    }
                }

                Write-Host ""
                Write-ColorOutput "  $([char]0x2713) = Enabled  $([char]0x2717) = Disabled" -ForegroundColor BrightBlack

                $enabledCount = ($startupItems | Where-Object { $_.Enabled }).Count
                $disabledCount = ($startupItems | Where-Object { -not $_.Enabled }).Count

                Write-Host ""
                Write-ColorOutput "  Total: $($startupItems.Count) items ($enabledCount enabled, $disabledCount disabled)" -ForegroundColor BrightBlack
            }

            'Analyze' {
                Write-WinMoleSection -Title "Boot Impact Analysis"
                Write-Host ""

                $enabledItems = $startupItems | Where-Object { $_.Enabled }
                $totalImpact = ($enabledItems | Measure-Object -Property EstimatedSeconds -Sum).Sum

                # Summary box
                $width = 54
                $top = [char]0x2554 + ([char]0x2550 * ($width - 2)) + [char]0x2557
                $mid = [char]0x2560 + ([char]0x2550 * ($width - 2)) + [char]0x2563
                $bot = [char]0x255A + ([char]0x2550 * ($width - 2)) + [char]0x255D
                $side = [char]0x2551

                $impactLevel = switch ($totalImpact) {
                    { $_ -gt 30 } { 'High' }
                    { $_ -gt 15 } { 'Medium' }
                    default { 'Low' }
                }

                $impactColor = switch ($impactLevel) {
                    'High' { 'Red' }
                    'Medium' { 'Yellow' }
                    default { 'Green' }
                }

                Write-ColorOutput $top -ForegroundColor Cyan
                Write-Host "$side  Boot Impact: $(Get-ColoredString "$impactLevel (estimated +$([math]::Round($totalImpact))s)" -ForegroundColor $impactColor)".PadRight($width - 1) + $side
                Write-ColorOutput $mid -ForegroundColor Cyan

                # Impact by category
                $byCategory = $enabledItems | Group-Object Category | ForEach-Object {
                    $catImpact = ($_.Group | Measure-Object -Property EstimatedSeconds -Sum).Sum
                    [PSCustomObject]@{
                        Category = $_.Name
                        Count = $_.Count
                        Impact = $catImpact
                    }
                } | Sort-Object Impact -Descending

                foreach ($cat in $byCategory) {
                    $percent = if ($totalImpact -gt 0) { [math]::Round(($cat.Impact / $totalImpact) * 100) } else { 0 }
                    $bar = Write-InlineProgressBar -Percent $percent -Width 15

                    $line = "  $($cat.Category.PadRight(15)) $bar $($cat.Count) items, $([math]::Round($cat.Impact))s"
                    Write-Host "$side$($line.PadRight($width - 2))$side"
                }

                Write-ColorOutput $bot -ForegroundColor Cyan

                # High impact items
                $highImpact = $enabledItems | Where-Object { $_.Impact -eq 'High' } | Sort-Object EstimatedSeconds -Descending

                if ($highImpact.Count -gt 0) {
                    Write-Host ""
                    Write-WinMoleSection -Title "High Impact Items (consider disabling)"

                    foreach ($item in $highImpact | Select-Object -First 5) {
                        Write-ColorOutput "    $([char]0x26A0) " -ForegroundColor Yellow -NoNewline
                        Write-ColorOutput "$($item.Name)" -ForegroundColor White -NoNewline
                        Write-ColorOutput " - $($item.EstimatedSeconds)s estimated" -ForegroundColor BrightBlack
                    }
                }

                # Non-essential items
                $nonEssential = $enabledItems | Where-Object { $_.Category -eq 'Third-party' -and $_.Impact -in @('High', 'Medium') }

                if ($nonEssential.Count -gt 0) {
                    Write-Host ""
                    Write-WinMoleStatus -Message "$($nonEssential.Count) third-party apps with Medium/High impact could be disabled" -Type Info
                }
            }

            'Disable' {
                if (-not $Name) {
                    Write-WinMoleStatus -Message "Specify a startup item name with -Name" -Type Error
                    return
                }

                $targets = $startupItems | Where-Object { $_.Name -like "*$Name*" -and $_.Enabled }

                if ($targets.Count -eq 0) {
                    Write-WinMoleStatus -Message "No enabled startup items matching '$Name'" -Type Warning
                    return
                }

                foreach ($target in $targets) {
                    Write-WinMoleStatus -Message "Disabling: $($target.Name)" -Type Progress

                    if ($PSCmdlet.ShouldProcess($target.Name, "Disable startup item")) {
                        try {
                            switch ($target.Source) {
                                'Registry' {
                                    # Add to StartupApproved\Run to disable
                                    $approvedPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run'
                                    if (-not (Test-Path $approvedPath)) {
                                        New-Item -Path $approvedPath -Force | Out-Null
                                    }

                                    # 03 00 00 00 ... = disabled
                                    $disabledValue = [byte[]](0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00)
                                    Set-ItemProperty -Path $approvedPath -Name $target.Name -Value $disabledValue -Type Binary

                                    Write-WinMoleStatus -Message "$($target.Name) disabled" -Type Success
                                }
                                'Startup Folder' {
                                    # Rename file to .disabled
                                    $newPath = "$($target.SourcePath).disabled"
                                    Rename-Item -LiteralPath $target.SourcePath -NewName (Split-Path $newPath -Leaf)
                                    Write-WinMoleStatus -Message "$($target.Name) disabled (renamed)" -Type Success
                                }
                                'Scheduled Task' {
                                    Disable-ScheduledTask -TaskPath $target.SourcePath -TaskName $target.Name -ErrorAction Stop
                                    Write-WinMoleStatus -Message "$($target.Name) disabled" -Type Success
                                }
                                default {
                                    Write-WinMoleStatus -Message "Cannot disable items from source: $($target.Source)" -Type Warning
                                }
                            }
                        }
                        catch {
                            Write-WinMoleStatus -Message "Failed to disable $($target.Name): $_" -Type Error
                        }
                    }
                }
            }

            'Enable' {
                if (-not $Name) {
                    Write-WinMoleStatus -Message "Specify a startup item name with -Name" -Type Error
                    return
                }

                $targets = $startupItems | Where-Object { $_.Name -like "*$Name*" -and -not $_.Enabled }

                if ($targets.Count -eq 0) {
                    Write-WinMoleStatus -Message "No disabled startup items matching '$Name'" -Type Warning
                    return
                }

                foreach ($target in $targets) {
                    Write-WinMoleStatus -Message "Enabling: $($target.Name)" -Type Progress

                    if ($PSCmdlet.ShouldProcess($target.Name, "Enable startup item")) {
                        try {
                            switch ($target.Source) {
                                'Registry' {
                                    $approvedPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run'

                                    # 02 00 00 00 ... = enabled
                                    $enabledValue = [byte[]](0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00)
                                    Set-ItemProperty -Path $approvedPath -Name $target.Name -Value $enabledValue -Type Binary

                                    Write-WinMoleStatus -Message "$($target.Name) enabled" -Type Success
                                }
                                'Startup Folder' {
                                    # Remove .disabled extension
                                    if ($target.SourcePath -match '\.disabled$') {
                                        $newPath = $target.SourcePath -replace '\.disabled$', ''
                                        Rename-Item -LiteralPath $target.SourcePath -NewName (Split-Path $newPath -Leaf)
                                        Write-WinMoleStatus -Message "$($target.Name) enabled" -Type Success
                                    }
                                }
                                'Scheduled Task' {
                                    Enable-ScheduledTask -TaskPath $target.SourcePath -TaskName $target.Name -ErrorAction Stop
                                    Write-WinMoleStatus -Message "$($target.Name) enabled" -Type Success
                                }
                                default {
                                    Write-WinMoleStatus -Message "Cannot enable items from source: $($target.Source)" -Type Warning
                                }
                            }
                        }
                        catch {
                            Write-WinMoleStatus -Message "Failed to enable $($target.Name): $_" -Type Error
                        }
                    }
                }
            }

            'Remove' {
                if (-not $Name) {
                    Write-WinMoleStatus -Message "Specify a startup item name with -Name" -Type Error
                    return
                }

                $targets = $startupItems | Where-Object { $_.Name -like "*$Name*" }

                if ($targets.Count -eq 0) {
                    Write-WinMoleStatus -Message "No startup items matching '$Name'" -Type Warning
                    return
                }

                foreach ($target in $targets) {
                    if (-not $Force) {
                        $confirm = Show-Confirmation -Message "Remove startup item '$($target.Name)'?"
                        if (-not $confirm) { continue }
                    }

                    if ($PSCmdlet.ShouldProcess($target.Name, "Remove startup item")) {
                        try {
                            switch ($target.Source) {
                                'Registry' {
                                    Remove-ItemProperty -Path $target.SourcePath -Name $target.Name -Force
                                    Write-WinMoleStatus -Message "$($target.Name) removed from registry" -Type Success
                                }
                                'Startup Folder' {
                                    Remove-Item -LiteralPath $target.SourcePath -Force
                                    Write-WinMoleStatus -Message "$($target.Name) removed from startup folder" -Type Success
                                }
                                default {
                                    Write-WinMoleStatus -Message "Cannot remove items from source: $($target.Source)" -Type Warning
                                }
                            }
                        }
                        catch {
                            Write-WinMoleStatus -Message "Failed to remove $($target.Name): $_" -Type Error
                        }
                    }
                }
            }
        }
    }

    end {
        Write-Host ""

        return $startupItems
    }
}
