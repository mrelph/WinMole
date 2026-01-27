function Invoke-WinMoleWinget {
    <#
    .SYNOPSIS
        Integrated winget package management.
    .DESCRIPTION
        Wrapper around winget with enhanced output formatting, batch operations,
        and update management.
    .PARAMETER Action
        The action to perform: list, update, search, install, uninstall, audit, export, import.
    .PARAMETER Package
        Package ID or name for install/uninstall/update operations.
    .PARAMETER Query
        Search query for search operation.
    .PARAMETER All
        Update all packages (for update action).
    .PARAMETER Source
        Package source (winget, msstore).
    .PARAMETER ExportPath
        Path for export operation.
    .PARAMETER ImportPath
        Path for import operation.
    .EXAMPLE
        Invoke-WinMoleWinget list
        List all installed packages.
    .EXAMPLE
        Invoke-WinMoleWinget update -All
        Update all packages.
    .EXAMPLE
        Invoke-WinMoleWinget audit
        Show outdated packages and security info.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory, Position = 0)]
        [ValidateSet('list', 'update', 'search', 'install', 'uninstall', 'audit', 'export', 'import')]
        [string]$Action,

        [Parameter(Position = 1)]
        [string]$Package,

        [Parameter()]
        [string]$Query,

        [Parameter()]
        [switch]$All,

        [Parameter()]
        [string]$Source = 'winget',

        [Parameter()]
        [string]$ExportPath,

        [Parameter()]
        [string]$ImportPath,

        [Parameter()]
        [switch]$Interactive
    )

    begin {
        # Check if winget is available
        $wingetPath = Get-Command 'winget' -ErrorAction SilentlyContinue
        if (-not $wingetPath) {
            Write-WinMoleStatus -Message "winget is not installed or not in PATH" -Type Error
            Write-ColorOutput "  Install from: https://aka.ms/getwinget" -ForegroundColor BrightBlack
            return
        }

        Write-WinMoleHeader -Title "WinMole Package Manager" -Style Box
        Write-Host ""
    }

    process {
        switch ($Action) {
            'list' {
                Write-WinMoleStatus -Message "Fetching installed packages..." -Type Progress

                $output = winget list --accept-source-agreements 2>$null
                $lines = $output -split "`n" | Where-Object { $_ -match '\S' }

                # Parse the output
                $packages = @()
                $headerFound = $false

                foreach ($line in $lines) {
                    if ($line -match '^Name\s+Id\s+Version') {
                        $headerFound = $true
                        continue
                    }
                    if ($line -match '^-+') { continue }
                    if (-not $headerFound) { continue }

                    # Parse package line
                    if ($line -match '^(.+?)\s{2,}(\S+)\s+(\S+)(?:\s+(\S+))?(?:\s+(\S+))?') {
                        $packages += [PSCustomObject]@{
                            Name = $Matches[1].Trim()
                            Id = $Matches[2]
                            Version = $Matches[3]
                            Available = if ($Matches[4] -and $Matches[4] -ne $Matches[3]) { $Matches[4] } else { $null }
                            Source = $Matches[5]
                        }
                    }
                }

                $outdated = $packages | Where-Object { $_.Available }

                Write-WinMoleSection -Title "Installed Packages ($($packages.Count))"
                Write-Host ""

                if ($packages.Count -gt 0) {
                    # Show with color-coding for outdated
                    foreach ($pkg in $packages | Select-Object -First 50) {
                        $statusIcon = if ($pkg.Available) {
                            Get-ColoredString -Text [char]0x26A0 -ForegroundColor Yellow
                        } else {
                            Get-ColoredString -Text [char]0x2713 -ForegroundColor Green
                        }

                        $nameStr = $pkg.Name
                        if ($nameStr.Length -gt 35) { $nameStr = $nameStr.Substring(0, 32) + '...' }

                        Write-Host "  $statusIcon " -NoNewline
                        Write-ColorOutput $nameStr.PadRight(35) -ForegroundColor White -NoNewline
                        Write-ColorOutput $pkg.Version.PadRight(15) -ForegroundColor BrightBlack -NoNewline

                        if ($pkg.Available) {
                            Write-ColorOutput " -> $($pkg.Available)" -ForegroundColor Yellow
                        } else {
                            Write-Host ""
                        }
                    }

                    if ($packages.Count -gt 50) {
                        Write-ColorOutput "  ... and $($packages.Count - 50) more" -ForegroundColor BrightBlack
                    }
                }

                Write-Host ""
                Write-ColorOutput "  $([char]0x2713) Up to date: $($packages.Count - $outdated.Count)" -ForegroundColor Green
                Write-ColorOutput "  $([char]0x26A0) Updates available: $($outdated.Count)" -ForegroundColor Yellow

                return $packages
            }

            'audit' {
                Write-WinMoleStatus -Message "Checking for updates..." -Type Progress

                $output = winget upgrade --accept-source-agreements 2>$null
                $lines = $output -split "`n" | Where-Object { $_ -match '\S' }

                $upgradable = @()
                $headerFound = $false

                foreach ($line in $lines) {
                    if ($line -match '^Name\s+Id\s+Version') {
                        $headerFound = $true
                        continue
                    }
                    if ($line -match '^-+') { continue }
                    if (-not $headerFound) { continue }
                    if ($line -match 'upgrades available') { continue }

                    if ($line -match '^(.+?)\s{2,}(\S+)\s+(\S+)\s+(\S+)') {
                        $upgradable += [PSCustomObject]@{
                            Name = $Matches[1].Trim()
                            Id = $Matches[2]
                            CurrentVersion = $Matches[3]
                            AvailableVersion = $Matches[4]
                        }
                    }
                }

                Write-WinMoleSection -Title "Package Audit"
                Write-Host ""

                if ($upgradable.Count -gt 0) {
                    Write-WinMoleStatus -Message "$($upgradable.Count) packages have updates available" -Type Warning
                    Write-Host ""

                    foreach ($pkg in $upgradable) {
                        $nameStr = $pkg.Name
                        if ($nameStr.Length -gt 30) { $nameStr = $nameStr.Substring(0, 27) + '...' }

                        Write-ColorOutput "  $([char]0x26A0) " -ForegroundColor Yellow -NoNewline
                        Write-ColorOutput $nameStr.PadRight(30) -ForegroundColor White -NoNewline
                        Write-ColorOutput "$($pkg.CurrentVersion) -> $($pkg.AvailableVersion)" -ForegroundColor Yellow
                    }

                    Write-Host ""
                    Write-ColorOutput "  Run 'Invoke-WinMoleWinget update -All' to update all packages" -ForegroundColor BrightBlack
                } else {
                    Write-WinMoleStatus -Message "All packages are up to date!" -Type Success
                }

                return $upgradable
            }

            'update' {
                if ($All) {
                    Write-WinMoleStatus -Message "Updating all packages..." -Type Progress
                    Write-Host ""

                    $process = Start-Process -FilePath 'winget' -ArgumentList 'upgrade', '--all', '--accept-source-agreements', '--accept-package-agreements' -NoNewWindow -PassThru -Wait

                    if ($process.ExitCode -eq 0) {
                        Write-WinMoleStatus -Message "All packages updated successfully" -Type Success
                    } else {
                        Write-WinMoleStatus -Message "Some packages may have failed to update" -Type Warning
                    }
                }
                elseif ($Package) {
                    Write-WinMoleStatus -Message "Updating $Package..." -Type Progress

                    $process = Start-Process -FilePath 'winget' -ArgumentList 'upgrade', $Package, '--accept-source-agreements', '--accept-package-agreements' -NoNewWindow -PassThru -Wait

                    if ($process.ExitCode -eq 0) {
                        Write-WinMoleStatus -Message "$Package updated successfully" -Type Success
                    } else {
                        Write-WinMoleStatus -Message "Failed to update $Package" -Type Error
                    }
                }
                else {
                    Write-WinMoleStatus -Message "Specify -Package or -All" -Type Error
                }
            }

            'search' {
                $searchQuery = if ($Query) { $Query } elseif ($Package) { $Package } else {
                    Write-WinMoleStatus -Message "Specify a search query" -Type Error
                    return
                }

                Write-WinMoleStatus -Message "Searching for '$searchQuery'..." -Type Progress

                $output = winget search $searchQuery --accept-source-agreements 2>$null
                $lines = $output -split "`n" | Where-Object { $_ -match '\S' }

                $results = @()
                $headerFound = $false

                foreach ($line in $lines) {
                    if ($line -match '^Name\s+Id\s+Version') {
                        $headerFound = $true
                        continue
                    }
                    if ($line -match '^-+') { continue }
                    if (-not $headerFound) { continue }

                    if ($line -match '^(.+?)\s{2,}(\S+)\s+(\S+)(?:\s+(.+))?') {
                        $results += [PSCustomObject]@{
                            Name = $Matches[1].Trim()
                            Id = $Matches[2]
                            Version = $Matches[3]
                            Source = if ($Matches[4]) { $Matches[4].Trim() } else { 'winget' }
                        }
                    }
                }

                Write-WinMoleSection -Title "Search Results ($($results.Count))"
                Write-Host ""

                if ($results.Count -gt 0) {
                    foreach ($pkg in $results | Select-Object -First 20) {
                        $nameStr = $pkg.Name
                        if ($nameStr.Length -gt 35) { $nameStr = $nameStr.Substring(0, 32) + '...' }

                        Write-ColorOutput "  $([char]0x25CF) " -ForegroundColor Cyan -NoNewline
                        Write-ColorOutput $nameStr.PadRight(35) -ForegroundColor White -NoNewline
                        Write-ColorOutput $pkg.Id -ForegroundColor BrightBlack
                    }

                    if ($results.Count -gt 20) {
                        Write-ColorOutput "  ... and $($results.Count - 20) more results" -ForegroundColor BrightBlack
                    }
                } else {
                    Write-WinMoleStatus -Message "No packages found" -Type Info
                }

                return $results
            }

            'install' {
                if (-not $Package) {
                    Write-WinMoleStatus -Message "Specify a package to install" -Type Error
                    return
                }

                Write-WinMoleStatus -Message "Installing $Package..." -Type Progress

                $args = @('install', $Package, '--accept-source-agreements', '--accept-package-agreements')
                if ($Interactive) { $args += '--interactive' }

                $process = Start-Process -FilePath 'winget' -ArgumentList $args -NoNewWindow -PassThru -Wait

                if ($process.ExitCode -eq 0) {
                    Write-WinMoleStatus -Message "$Package installed successfully" -Type Success
                } else {
                    Write-WinMoleStatus -Message "Failed to install $Package" -Type Error
                }
            }

            'uninstall' {
                if (-not $Package) {
                    Write-WinMoleStatus -Message "Specify a package to uninstall" -Type Error
                    return
                }

                Write-WinMoleStatus -Message "Uninstalling $Package..." -Type Progress

                $process = Start-Process -FilePath 'winget' -ArgumentList 'uninstall', $Package -NoNewWindow -PassThru -Wait

                if ($process.ExitCode -eq 0) {
                    Write-WinMoleStatus -Message "$Package uninstalled successfully" -Type Success
                } else {
                    Write-WinMoleStatus -Message "Failed to uninstall $Package" -Type Error
                }
            }

            'export' {
                $exportFile = if ($ExportPath) { $ExportPath } else {
                    Join-Path ([Environment]::GetFolderPath('Desktop')) "winmole-packages-$(Get-Date -Format 'yyyyMMdd').json"
                }

                Write-WinMoleStatus -Message "Exporting package list..." -Type Progress

                winget export -o $exportFile --accept-source-agreements 2>$null

                if (Test-Path $exportFile) {
                    Write-WinMoleStatus -Message "Exported to: $exportFile" -Type Success
                } else {
                    Write-WinMoleStatus -Message "Export failed" -Type Error
                }
            }

            'import' {
                if (-not $ImportPath -or -not (Test-Path $ImportPath)) {
                    Write-WinMoleStatus -Message "Specify a valid import file path" -Type Error
                    return
                }

                Write-WinMoleStatus -Message "Importing packages from $ImportPath..." -Type Progress

                $process = Start-Process -FilePath 'winget' -ArgumentList 'import', '-i', $ImportPath, '--accept-source-agreements', '--accept-package-agreements' -NoNewWindow -PassThru -Wait

                if ($process.ExitCode -eq 0) {
                    Write-WinMoleStatus -Message "Import completed successfully" -Type Success
                } else {
                    Write-WinMoleStatus -Message "Some packages may have failed to import" -Type Warning
                }
            }
        }
    }
}
