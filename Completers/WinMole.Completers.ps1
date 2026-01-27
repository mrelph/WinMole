# WinMole Tab Completion

# Register argument completers for WinMole cmdlets

# Invoke-WinMoleClean -Category completer
Register-ArgumentCompleter -CommandName 'Invoke-WinMoleClean' -ParameterName 'Category' -ScriptBlock {
    param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)

    @('All', 'User', 'System', 'Browser', 'Windows', 'Cache') |
        Where-Object { $_ -like "$wordToComplete*" } |
        ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
}

# Get-WinMoleDisk -Mode completer
Register-ArgumentCompleter -CommandName 'Get-WinMoleDisk' -ParameterName 'Mode' -ScriptBlock {
    param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)

    @(
        @{ Name = 'Tree'; Description = 'Tree-view disk usage visualization' }
        @{ Name = 'LargestFiles'; Description = 'Find largest files' }
        @{ Name = 'LargestFolders'; Description = 'Find largest folders' }
        @{ Name = 'Duplicates'; Description = 'Find duplicate files' }
        @{ Name = 'FileTypes'; Description = 'File type breakdown' }
        @{ Name = 'OldFiles'; Description = 'Find old files' }
        @{ Name = 'Summary'; Description = 'Drive summary' }
    ) | Where-Object { $_.Name -like "$wordToComplete*" } |
        ForEach-Object {
            [System.Management.Automation.CompletionResult]::new(
                $_.Name, $_.Name, 'ParameterValue', $_.Description
            )
        }
}

# Get-WinMoleDisk -Path completer (directories)
Register-ArgumentCompleter -CommandName 'Get-WinMoleDisk' -ParameterName 'Path' -ScriptBlock {
    param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)

    Get-ChildItem -Path "$wordToComplete*" -Directory -ErrorAction SilentlyContinue |
        ForEach-Object {
            $path = $_.FullName
            if ($path -match '\s') { $path = "`"$path`"" }
            [System.Management.Automation.CompletionResult]::new(
                $path, $_.Name, 'ParameterValue', $_.FullName
            )
        }
}

# Clear-WinMoleDevArtifacts -Type completer
Register-ArgumentCompleter -CommandName 'Clear-WinMoleDevArtifacts' -ParameterName 'Type' -ScriptBlock {
    param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)

    @(
        @{ Name = 'node_modules'; Description = 'Node.js dependencies' }
        @{ Name = 'bin'; Description = '.NET build output' }
        @{ Name = 'obj'; Description = '.NET intermediate files' }
        @{ Name = 'target'; Description = 'Rust/Cargo build output' }
        @{ Name = '__pycache__'; Description = 'Python bytecode cache' }
        @{ Name = '.pytest_cache'; Description = 'Pytest cache' }
        @{ Name = '.gradle'; Description = 'Gradle cache' }
        @{ Name = '.nuget'; Description = 'NuGet packages' }
        @{ Name = 'vendor'; Description = 'PHP/Go dependencies' }
        @{ Name = 'packages'; Description = 'NuGet packages (solution)' }
        @{ Name = '.next'; Description = 'Next.js build' }
        @{ Name = '.nuxt'; Description = 'Nuxt.js build' }
        @{ Name = 'dist'; Description = 'Distribution output' }
        @{ Name = 'build'; Description = 'Build output' }
        @{ Name = 'All'; Description = 'All artifact types' }
    ) | Where-Object { $_.Name -like "$wordToComplete*" } |
        ForEach-Object {
            [System.Management.Automation.CompletionResult]::new(
                $_.Name, $_.Name, 'ParameterValue', $_.Description
            )
        }
}

# Invoke-WinMoleWinget -Action completer
Register-ArgumentCompleter -CommandName 'Invoke-WinMoleWinget' -ParameterName 'Action' -ScriptBlock {
    param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)

    @(
        @{ Name = 'list'; Description = 'List installed packages' }
        @{ Name = 'update'; Description = 'Update packages' }
        @{ Name = 'search'; Description = 'Search for packages' }
        @{ Name = 'install'; Description = 'Install a package' }
        @{ Name = 'uninstall'; Description = 'Remove a package' }
        @{ Name = 'audit'; Description = 'Check for updates' }
        @{ Name = 'export'; Description = 'Export package list' }
        @{ Name = 'import'; Description = 'Import packages' }
    ) | Where-Object { $_.Name -like "$wordToComplete*" } |
        ForEach-Object {
            [System.Management.Automation.CompletionResult]::new(
                $_.Name, $_.Name, 'ParameterValue', $_.Description
            )
        }
}

# Invoke-WinMoleRegistry -Mode completer
Register-ArgumentCompleter -CommandName 'Invoke-WinMoleRegistry' -ParameterName 'Mode' -ScriptBlock {
    param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)

    @(
        @{ Name = 'Scan'; Description = 'Scan for issues (default)' }
        @{ Name = 'Clean'; Description = 'Clean found issues' }
    ) | Where-Object { $_.Name -like "$wordToComplete*" } |
        ForEach-Object {
            [System.Management.Automation.CompletionResult]::new(
                $_.Name, $_.Name, 'ParameterValue', $_.Description
            )
        }
}

# Invoke-WinMoleRegistry -Category completer
Register-ArgumentCompleter -CommandName 'Invoke-WinMoleRegistry' -ParameterName 'Category' -ScriptBlock {
    param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)

    @(
        @{ Name = 'InvalidPaths'; Description = 'Invalid file/path references' }
        @{ Name = 'MissingDLLs'; Description = 'Missing shared DLLs' }
        @{ Name = 'OrphanedSoftware'; Description = 'Uninstalled program entries' }
        @{ Name = 'InvalidExtensions'; Description = 'Invalid file associations' }
        @{ Name = 'EmptyKeys'; Description = 'Empty registry keys' }
        @{ Name = 'All'; Description = 'All categories' }
    ) | Where-Object { $_.Name -like "$wordToComplete*" } |
        ForEach-Object {
            [System.Management.Automation.CompletionResult]::new(
                $_.Name, $_.Name, 'ParameterValue', $_.Description
            )
        }
}

# Optimize-WinMoleStartup -Action completer
Register-ArgumentCompleter -CommandName 'Optimize-WinMoleStartup' -ParameterName 'Action' -ScriptBlock {
    param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)

    @(
        @{ Name = 'List'; Description = 'List all startup items' }
        @{ Name = 'Disable'; Description = 'Disable a startup item' }
        @{ Name = 'Enable'; Description = 'Enable a startup item' }
        @{ Name = 'Remove'; Description = 'Remove a startup item' }
        @{ Name = 'Analyze'; Description = 'Boot impact analysis' }
    ) | Where-Object { $_.Name -like "$wordToComplete*" } |
        ForEach-Object {
            [System.Management.Automation.CompletionResult]::new(
                $_.Name, $_.Name, 'ParameterValue', $_.Description
            )
        }
}

# Optimize-WinMoleStartup -Source completer
Register-ArgumentCompleter -CommandName 'Optimize-WinMoleStartup' -ParameterName 'Source' -ScriptBlock {
    param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)

    @(
        @{ Name = 'All'; Description = 'All sources' }
        @{ Name = 'Registry'; Description = 'Registry Run keys' }
        @{ Name = 'StartupFolder'; Description = 'Startup folder shortcuts' }
        @{ Name = 'ScheduledTask'; Description = 'Scheduled tasks with logon trigger' }
        @{ Name = 'Service'; Description = 'Auto-start services' }
    ) | Where-Object { $_.Name -like "$wordToComplete*" } |
        ForEach-Object {
            [System.Management.Automation.CompletionResult]::new(
                $_.Name, $_.Name, 'ParameterValue', $_.Description
            )
        }
}
