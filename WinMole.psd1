@{
    # Module manifest for WinMole
    RootModule = 'WinMole.psm1'
    ModuleVersion = '1.0.0'
    GUID = 'a8c9d4e5-f6b7-4c8d-9e0f-1a2b3c4d5e6f'
    Author = 'WinMole Contributors'
    CompanyName = 'Community'
    Copyright = '(c) 2025 WinMole Contributors. MIT License.'
    Description = 'Windows System Optimization CLI - Clean, analyze, and optimize your Windows system with a rich terminal UI.'

    # Minimum PowerShell version
    PowerShellVersion = '7.0'

    # Compatible PSEditions
    CompatiblePSEditions = @('Core')

    # Functions to export
    FunctionsToExport = @(
        'Invoke-WinMoleClean',
        'Get-WinMoleDisk',
        'Get-WinMoleStatus',
        'Clear-WinMoleDevArtifacts',
        'Invoke-WinMoleWinget',
        'Invoke-WinMoleRegistry',
        'Optimize-WinMoleStartup',
        'Show-WinMole'
    )

    # Cmdlets to export
    CmdletsToExport = @()

    # Variables to export
    VariablesToExport = @()

    # Aliases to export
    AliasesToExport = @(
        'winmole',
        'wm-clean',
        'wm-disk',
        'wm-status',
        'wm-dev',
        'wm-winget',
        'wm-registry',
        'wm-startup'
    )

    # Private data
    PrivateData = @{
        PSData = @{
            Tags = @('Windows', 'System', 'Optimization', 'Cleanup', 'CLI', 'Utility')
            LicenseUri = 'https://github.com/winmole/winmole/blob/main/LICENSE'
            ProjectUri = 'https://github.com/winmole/winmole'
            IconUri = ''
            ReleaseNotes = 'Initial release of WinMole - Windows System Optimization CLI'
        }
    }
}
