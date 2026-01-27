function ConvertTo-SafePath {
    <#
    .SYNOPSIS
        Converts a path to a safe, normalized format.
    .DESCRIPTION
        Handles environment variables, relative paths, and ensures
        the path is in a consistent format for processing.
    .PARAMETER Path
        The path to normalize.
    .PARAMETER Resolve
        Attempt to resolve the path to an absolute path.
    .PARAMETER MustExist
        Throw an error if the path doesn't exist.
    #>
    [CmdletBinding()]
    [OutputType([string])]
    param(
        [Parameter(Mandatory, Position = 0, ValueFromPipeline)]
        [AllowEmptyString()]
        [string]$Path,

        [Parameter()]
        [switch]$Resolve,

        [Parameter()]
        [switch]$MustExist
    )

    process {
        if ([string]::IsNullOrWhiteSpace($Path)) {
            if ($MustExist) {
                throw "Path cannot be empty"
            }
            return $null
        }

        # Expand environment variables
        $expanded = [Environment]::ExpandEnvironmentVariables($Path)

        # Handle PowerShell drive notation
        if ($expanded -match '^~[/\\]?') {
            $expanded = $expanded -replace '^~[/\\]?', "$([Environment]::GetFolderPath('UserProfile'))\"
        }

        # Normalize slashes
        $normalized = $expanded -replace '/', '\'

        # Remove trailing slash (unless root)
        if ($normalized -match '^[A-Z]:\\$') {
            # Keep trailing slash for drive root
        }
        elseif ($normalized -match '\\$') {
            $normalized = $normalized.TrimEnd('\')
        }

        # Resolve to absolute path if requested
        if ($Resolve) {
            try {
                $resolved = [System.IO.Path]::GetFullPath($normalized)
                $normalized = $resolved
            }
            catch {
                # Keep original if resolution fails
            }
        }

        # Check existence if required
        if ($MustExist -and -not (Test-Path -LiteralPath $normalized)) {
            throw "Path does not exist: $normalized"
        }

        return $normalized
    }
}

function Test-SafePath {
    <#
    .SYNOPSIS
        Tests if a path is safe to delete.
    .DESCRIPTION
        Checks if a path is a system-critical location that should not be deleted.
    .PARAMETER Path
        The path to test.
    #>
    [CmdletBinding()]
    [OutputType([bool])]
    param(
        [Parameter(Mandatory, Position = 0)]
        [string]$Path
    )

    # Normalize the path
    $normalizedPath = ConvertTo-SafePath -Path $Path -Resolve

    if ([string]::IsNullOrWhiteSpace($normalizedPath)) {
        return $false
    }

    # Critical paths that should never be deleted
    $protectedPaths = @(
        'C:\Windows',
        'C:\Windows\System32',
        'C:\Windows\SysWOW64',
        'C:\Program Files',
        'C:\Program Files (x86)',
        'C:\Users',
        'C:\ProgramData',
        $env:SystemRoot,
        $env:windir,
        $env:ProgramFiles,
        ${env:ProgramFiles(x86)},
        $env:ProgramData,
        $env:USERPROFILE,
        [Environment]::GetFolderPath('System'),
        [Environment]::GetFolderPath('Windows'),
        [Environment]::GetFolderPath('ProgramFiles'),
        [Environment]::GetFolderPath('UserProfile')
    ) | Where-Object { $_ } | ForEach-Object {
        ConvertTo-SafePath -Path $_ -Resolve -ErrorAction SilentlyContinue
    } | Where-Object { $_ }

    # Check if path is or is parent of protected path
    foreach ($protected in $protectedPaths) {
        if ($normalizedPath -eq $protected) {
            return $false
        }

        # Check if trying to delete a parent of a protected path
        if ($protected.StartsWith($normalizedPath + '\', [StringComparison]::OrdinalIgnoreCase)) {
            return $false
        }
    }

    # Additional checks for root directories
    if ($normalizedPath -match '^[A-Z]:\\?$') {
        return $false  # Never delete drive root
    }

    return $true
}

function Get-SafeDeletePath {
    <#
    .SYNOPSIS
        Gets the contents of a path that are safe to delete.
    .DESCRIPTION
        Returns items within a path that can be safely deleted,
        excluding system files and protected items.
    .PARAMETER Path
        The path to analyze.
    .PARAMETER Pattern
        Optional file pattern to match.
    .PARAMETER OlderThan
        Only include items older than this date.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory, Position = 0)]
        [string]$Path,

        [Parameter()]
        [string]$Pattern,

        [Parameter()]
        [datetime]$OlderThan
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        return @()
    }

    $params = @{
        Path = $Path
        Force = $true
        ErrorAction = 'SilentlyContinue'
    }

    if ($Pattern) {
        $params.Filter = $Pattern
    }

    $items = Get-ChildItem @params

    if ($OlderThan) {
        $items = $items | Where-Object { $_.LastWriteTime -lt $OlderThan }
    }

    # Filter out protected items
    $safeItems = $items | Where-Object {
        $fullPath = $_.FullName

        # Skip protected system files
        if ($_.Attributes -band [System.IO.FileAttributes]::System) {
            return $false
        }

        # Skip critical file names
        $protectedNames = @('desktop.ini', 'thumbs.db', 'ntuser.dat', 'pagefile.sys', 'hiberfil.sys', 'swapfile.sys')
        if ($_.Name.ToLower() -in $protectedNames) {
            return $false
        }

        return Test-SafePath -Path $fullPath
    }

    return $safeItems
}

function Remove-SafeItem {
    <#
    .SYNOPSIS
        Safely removes an item with validation.
    .DESCRIPTION
        Removes a file or folder after validating it's safe to delete.
    .PARAMETER Path
        The path to remove.
    .PARAMETER Force
        Skip confirmation.
    .PARAMETER WhatIf
        Preview without deleting.
    #>
    [CmdletBinding(SupportsShouldProcess)]
    param(
        [Parameter(Mandatory, Position = 0, ValueFromPipeline)]
        [string]$Path,

        [Parameter()]
        [switch]$Force
    )

    process {
        if (-not (Test-SafePath -Path $Path)) {
            Write-Warning "Refusing to delete protected path: $Path"
            return $false
        }

        if ($PSCmdlet.ShouldProcess($Path, "Remove")) {
            try {
                Remove-Item -LiteralPath $Path -Recurse -Force -ErrorAction Stop
                return $true
            }
            catch {
                Write-Warning "Failed to remove: $Path - $_"
                return $false
            }
        }

        return $false
    }
}
