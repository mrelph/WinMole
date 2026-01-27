function Clear-WinMoleDevArtifacts {
    <#
    .SYNOPSIS
        Remove development build artifacts to reclaim space.
    .DESCRIPTION
        Scans for and removes common development artifacts like node_modules,
        .nuget packages, bin/obj folders, target folders, and more.
    .PARAMETER Path
        The root path to search. Defaults to current directory.
    .PARAMETER Type
        Artifact types to target. Default is common development folders.
    .PARAMETER OlderThan
        Only remove artifacts not modified in X days.
    .PARAMETER WhatIf
        Preview without deleting.
    .PARAMETER Force
        Skip confirmation prompts.
    .PARAMETER MaxDepth
        Maximum directory depth to search.
    .EXAMPLE
        Clear-WinMoleDevArtifacts -Path ~/Code -WhatIf
        Preview cleanup of dev artifacts in Code folder.
    .EXAMPLE
        Clear-WinMoleDevArtifacts -Type node_modules,target -OlderThan 30
        Remove node_modules and Rust target folders older than 30 days.
    #>
    [CmdletBinding(SupportsShouldProcess, ConfirmImpact = 'High')]
    param(
        [Parameter(Position = 0)]
        [string]$Path = '.',

        [Parameter()]
        [ValidateSet('node_modules', 'bin', 'obj', 'target', '__pycache__', '.pytest_cache',
                     '.gradle', '.nuget', 'vendor', 'packages', 'bower_components', '.next',
                     '.nuxt', 'dist', 'build', '.tox', '.mypy_cache', 'coverage', 'All')]
        [string[]]$Type = @('node_modules', 'bin', 'obj', '__pycache__', 'target'),

        [Parameter()]
        [int]$OlderThan = 0,

        [Parameter()]
        [switch]$Force,

        [Parameter()]
        [int]$MaxDepth = 10,

        [Parameter()]
        [Alias('DryRun')]
        [switch]$Preview
    )

    begin {
        $resolvedPath = ConvertTo-SafePath -Path $Path -Resolve -MustExist
        $isPreview = $Preview -or $WhatIfPreference

        # Artifact definitions with context
        $artifactDefs = @{
            'node_modules' = @{
                Pattern = 'node_modules'
                Description = 'Node.js dependencies'
                Indicator = 'package.json'
                TypicalSize = 'Large'
            }
            'bin' = @{
                Pattern = 'bin'
                Description = '.NET build output'
                Indicator = '*.csproj', '*.fsproj', '*.vbproj'
                TypicalSize = 'Medium'
            }
            'obj' = @{
                Pattern = 'obj'
                Description = '.NET intermediate files'
                Indicator = '*.csproj', '*.fsproj', '*.vbproj'
                TypicalSize = 'Medium'
            }
            'target' = @{
                Pattern = 'target'
                Description = 'Rust/Cargo build output'
                Indicator = 'Cargo.toml'
                TypicalSize = 'Very Large'
            }
            '__pycache__' = @{
                Pattern = '__pycache__'
                Description = 'Python bytecode cache'
                Indicator = '*.py'
                TypicalSize = 'Small'
            }
            '.pytest_cache' = @{
                Pattern = '.pytest_cache'
                Description = 'Pytest cache'
                Indicator = 'pytest.ini', 'setup.py'
                TypicalSize = 'Small'
            }
            '.gradle' = @{
                Pattern = '.gradle'
                Description = 'Gradle cache'
                Indicator = 'build.gradle', 'settings.gradle'
                TypicalSize = 'Large'
            }
            '.nuget' = @{
                Pattern = '.nuget'
                Description = 'NuGet packages (local)'
                Indicator = '*.sln'
                TypicalSize = 'Large'
            }
            'vendor' = @{
                Pattern = 'vendor'
                Description = 'PHP Composer / Go dependencies'
                Indicator = 'composer.json', 'go.mod'
                TypicalSize = 'Medium'
            }
            'packages' = @{
                Pattern = 'packages'
                Description = 'NuGet packages (solution level)'
                Indicator = '*.sln'
                TypicalSize = 'Large'
            }
            'bower_components' = @{
                Pattern = 'bower_components'
                Description = 'Bower dependencies (legacy)'
                Indicator = 'bower.json'
                TypicalSize = 'Medium'
            }
            '.next' = @{
                Pattern = '.next'
                Description = 'Next.js build output'
                Indicator = 'next.config.js'
                TypicalSize = 'Large'
            }
            '.nuxt' = @{
                Pattern = '.nuxt'
                Description = 'Nuxt.js build output'
                Indicator = 'nuxt.config.js'
                TypicalSize = 'Medium'
            }
            'dist' = @{
                Pattern = 'dist'
                Description = 'Distribution/build output'
                Indicator = 'package.json', 'setup.py'
                TypicalSize = 'Medium'
            }
            'build' = @{
                Pattern = 'build'
                Description = 'Build output (various)'
                Indicator = 'CMakeLists.txt', 'Makefile'
                TypicalSize = 'Medium'
            }
            '.tox' = @{
                Pattern = '.tox'
                Description = 'Tox test environments'
                Indicator = 'tox.ini'
                TypicalSize = 'Large'
            }
            '.mypy_cache' = @{
                Pattern = '.mypy_cache'
                Description = 'MyPy type checker cache'
                Indicator = 'mypy.ini', '*.py'
                TypicalSize = 'Small'
            }
            'coverage' = @{
                Pattern = 'coverage'
                Description = 'Code coverage reports'
                Indicator = '.coveragerc', 'jest.config.js'
                TypicalSize = 'Small'
            }
        }

        Write-WinMoleHeader -Title "WinMole Developer Cleanup" -Style Box

        if ($isPreview) {
            Write-WinMoleStatus -Message "Running in PREVIEW mode - no folders will be deleted" -Type Warning
        }

        Write-Host ""
        Write-ColorOutput "  Scanning: $resolvedPath" -ForegroundColor BrightBlack
        Write-ColorOutput "  Types: $($Type -join ', ')" -ForegroundColor BrightBlack
        if ($OlderThan -gt 0) {
            Write-ColorOutput "  Older than: $OlderThan days" -ForegroundColor BrightBlack
        }
        Write-Host ""
    }

    process {
        $targetTypes = if ($Type -contains 'All') { $artifactDefs.Keys } else { $Type }
        $cutoffDate = if ($OlderThan -gt 0) { (Get-Date).AddDays(-$OlderThan) } else { $null }

        $foundArtifacts = @()
        $totalSize = 0
        $totalCount = 0

        foreach ($artifactType in $targetTypes) {
            $def = $artifactDefs[$artifactType]
            if (-not $def) { continue }

            Write-WinMoleStatus -Message "Scanning for $($def.Description)..." -Type Progress

            # Find matching folders
            $folders = Get-ChildItem -Path $resolvedPath -Directory -Recurse -Force -ErrorAction SilentlyContinue -Depth $MaxDepth |
                Where-Object { $_.Name -eq $def.Pattern }

            foreach ($folder in $folders) {
                # Check age if specified
                if ($cutoffDate -and $folder.LastWriteTime -gt $cutoffDate) {
                    continue
                }

                # Calculate size
                $size = 0
                try {
                    $size = (Get-ChildItem -Path $folder.FullName -Recurse -File -Force -ErrorAction SilentlyContinue |
                        Measure-Object -Property Length -Sum).Sum
                }
                catch { }

                if ($size -eq $null) { $size = 0 }

                $foundArtifacts += [PSCustomObject]@{
                    Type = $artifactType
                    Path = $folder.FullName
                    RelativePath = $folder.FullName -replace [regex]::Escape($resolvedPath), '.'
                    Size = $size
                    LastModified = $folder.LastWriteTime
                    Description = $def.Description
                }

                $totalSize += $size
                $totalCount++
            }
        }

        if ($foundArtifacts.Count -eq 0) {
            Write-WinMoleStatus -Message "No artifacts found matching criteria" -Type Info
            return
        }

        # Display findings grouped by type
        Write-WinMoleSection -Title "Found Artifacts"
        Write-Host ""

        $groupedArtifacts = $foundArtifacts | Group-Object Type

        foreach ($group in $groupedArtifacts | Sort-Object { ($_.Group | Measure-Object Size -Sum).Sum } -Descending) {
            $groupSize = ($group.Group | Measure-Object Size -Sum).Sum
            $def = $artifactDefs[$group.Name]

            Write-ColorOutput "  $($def.Description)" -ForegroundColor BrightCyan -NoNewline
            Write-ColorOutput " ($($group.Count) folders, $(Format-FileSize $groupSize))" -ForegroundColor BrightBlack
            Write-Host ""

            # Show top 5 largest of each type
            $topItems = $group.Group | Sort-Object Size -Descending | Select-Object -First 5

            foreach ($item in $topItems) {
                $sizeStr = (Format-FileSize $item.Size).PadLeft(10)
                $ageStr = [math]::Round(((Get-Date) - $item.LastModified).TotalDays)

                Write-ColorOutput "    $([char]0x25CF) " -ForegroundColor BrightBlack -NoNewline
                Write-ColorOutput $sizeStr -ForegroundColor White -NoNewline
                Write-ColorOutput " ($ageStr days)" -ForegroundColor BrightBlack -NoNewline
                Write-ColorOutput " $($item.RelativePath)" -ForegroundColor BrightBlack
            }

            if ($group.Count -gt 5) {
                $remaining = $group.Count - 5
                $remainingSize = ($group.Group | Select-Object -Skip 5 | Measure-Object Size -Sum).Sum
                Write-ColorOutput "    ... and $remaining more ($(Format-FileSize $remainingSize))" -ForegroundColor BrightBlack
            }
            Write-Host ""
        }

        # Summary
        Write-WinMoleSection -Title "Summary"
        Write-Host ""
        Write-ColorOutput "  Total artifacts: " -ForegroundColor White -NoNewline
        Write-ColorOutput $totalCount -ForegroundColor BrightCyan
        Write-ColorOutput "  Total size: " -ForegroundColor White -NoNewline
        Write-ColorOutput (Format-FileSize $totalSize) -ForegroundColor BrightCyan -Style Bold

        # Confirm and delete
        if (-not $isPreview) {
            Write-Host ""

            if (-not $Force) {
                $confirm = Show-Confirmation -Message "Delete all $totalCount artifact folders ($(Format-FileSize $totalSize))?"
                if (-not $confirm) {
                    Write-WinMoleStatus -Message "Cleanup cancelled" -Type Info
                    return
                }
            }

            Write-Host ""
            Write-WinMoleStatus -Message "Deleting artifacts..." -Type Progress

            $deletedCount = 0
            $deletedSize = 0
            $errorCount = 0

            foreach ($artifact in $foundArtifacts) {
                if ($PSCmdlet.ShouldProcess($artifact.Path, "Delete artifact folder")) {
                    try {
                        Remove-Item -LiteralPath $artifact.Path -Recurse -Force -ErrorAction Stop
                        $deletedCount++
                        $deletedSize += $artifact.Size
                    }
                    catch {
                        $errorCount++
                        Write-Verbose "Failed to delete $($artifact.Path): $_"
                    }
                }
            }

            Write-Host ""
            Write-WinMoleStatus -Message "Deleted $deletedCount folders, freed $(Format-FileSize $deletedSize)" -Type Success

            if ($errorCount -gt 0) {
                Write-WinMoleStatus -Message "$errorCount folders could not be deleted (may be in use)" -Type Warning
            }

            return [PSCustomObject]@{
                DeletedCount = $deletedCount
                DeletedSize = $deletedSize
                DeletedSizeFormatted = Format-FileSize $deletedSize
                ErrorCount = $errorCount
            }
        }
        else {
            Write-Host ""
            Write-WinMoleStatus -Message "Run without -WhatIf/-Preview to delete artifacts" -Type Info

            return [PSCustomObject]@{
                WouldDeleteCount = $totalCount
                WouldDeleteSize = $totalSize
                WouldDeleteSizeFormatted = Format-FileSize $totalSize
                Artifacts = $foundArtifacts
            }
        }
    }
}
