#Requires -Version 7.0
#Requires -PSEdition Core

<#
.SYNOPSIS
    WinMole - Windows System Optimization CLI
.DESCRIPTION
    A comprehensive CLI utility for Windows system optimization, cleaning, and analysis.
    Consolidates system maintenance tasks into a single PowerShell module with a rich terminal UI.
#>

# Module-level variables
$script:WinMoleVersion = '1.0.0'
$script:WinMoleConfigPath = Join-Path $PSScriptRoot 'Config\defaults.json'
$script:ModuleRoot = $PSScriptRoot

# ANSI escape code support check
$script:SupportsAnsi = $true
if ($env:NO_COLOR -or $env:TERM -eq 'dumb') {
    $script:SupportsAnsi = $false
}

# Import private functions first (order matters for dependencies)
$PrivateFunctions = @(
    'Private\UI\Write-ColorOutput.ps1',
    'Private\UI\Write-ProgressBar.ps1',
    'Private\UI\Write-Table.ps1',
    'Private\UI\Write-Tree.ps1',
    'Private\UI\Show-Menu.ps1',
    'Private\System\Test-AdminRights.ps1',
    'Private\System\Get-TempFolders.ps1',
    'Private\System\Get-BrowserCachePaths.ps1',
    'Private\System\Get-HealthScore.ps1',
    'Private\System\Get-StartupItems.ps1',
    'Private\System\Test-RegistryPath.ps1',
    'Private\Utils\Format-FileSize.ps1',
    'Private\Utils\Get-FileHash.ps1',
    'Private\Utils\ConvertTo-SafePath.ps1',
    'Private\Config\Get-WinMoleConfig.ps1'
)

foreach ($function in $PrivateFunctions) {
    $path = Join-Path $PSScriptRoot $function
    if (Test-Path $path) {
        . $path
    }
}

# Import public functions
$PublicFunctions = @(
    'Public\Invoke-WinMoleClean.ps1',
    'Public\Get-WinMoleDisk.ps1',
    'Public\Get-WinMoleStatus.ps1',
    'Public\Clear-WinMoleDevArtifacts.ps1',
    'Public\Invoke-WinMoleWinget.ps1',
    'Public\Invoke-WinMoleRegistry.ps1',
    'Public\Optimize-WinMoleStartup.ps1',
    'Public\Show-WinMole.ps1'
)

foreach ($function in $PublicFunctions) {
    $path = Join-Path $PSScriptRoot $function
    if (Test-Path $path) {
        . $path
    }
}

# Import tab completers
$CompleterPath = Join-Path $PSScriptRoot 'Completers\WinMole.Completers.ps1'
if (Test-Path $CompleterPath) {
    . $CompleterPath
}

# Set up aliases
Set-Alias -Name 'winmole' -Value 'Show-WinMole' -Scope Global
Set-Alias -Name 'wm-clean' -Value 'Invoke-WinMoleClean' -Scope Global
Set-Alias -Name 'wm-disk' -Value 'Get-WinMoleDisk' -Scope Global
Set-Alias -Name 'wm-status' -Value 'Get-WinMoleStatus' -Scope Global
Set-Alias -Name 'wm-dev' -Value 'Clear-WinMoleDevArtifacts' -Scope Global
Set-Alias -Name 'wm-winget' -Value 'Invoke-WinMoleWinget' -Scope Global
Set-Alias -Name 'wm-registry' -Value 'Invoke-WinMoleRegistry' -Scope Global
Set-Alias -Name 'wm-startup' -Value 'Optimize-WinMoleStartup' -Scope Global

# Export functions and aliases
Export-ModuleMember -Function @(
    'Invoke-WinMoleClean',
    'Get-WinMoleDisk',
    'Get-WinMoleStatus',
    'Clear-WinMoleDevArtifacts',
    'Invoke-WinMoleWinget',
    'Invoke-WinMoleRegistry',
    'Optimize-WinMoleStartup',
    'Show-WinMole'
) -Alias @(
    'winmole',
    'wm-clean',
    'wm-disk',
    'wm-status',
    'wm-dev',
    'wm-winget',
    'wm-registry',
    'wm-startup'
)
