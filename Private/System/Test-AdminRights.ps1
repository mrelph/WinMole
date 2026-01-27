function Test-AdminRights {
    <#
    .SYNOPSIS
        Tests if the current session has administrator privileges.
    .DESCRIPTION
        Checks whether the current PowerShell session is running with elevated
        (administrator) privileges. Returns $true if elevated, $false otherwise.
    .PARAMETER Quiet
        Don't output any messages, just return the boolean.
    .PARAMETER Required
        If specified and not admin, throws an error instead of returning $false.
    .EXAMPLE
        if (Test-AdminRights) { "Running as admin" }
    .EXAMPLE
        Test-AdminRights -Required  # Throws if not admin
    #>
    [CmdletBinding()]
    [OutputType([bool])]
    param(
        [Parameter()]
        [switch]$Quiet,

        [Parameter()]
        [switch]$Required
    )

    $isAdmin = $false

    if ($IsWindows -or $env:OS -eq 'Windows_NT') {
        # Windows: Check if running as administrator
        $currentPrincipal = New-Object Security.Principal.WindowsPrincipal(
            [Security.Principal.WindowsIdentity]::GetCurrent()
        )
        $isAdmin = $currentPrincipal.IsInRole(
            [Security.Principal.WindowsBuiltInRole]::Administrator
        )
    } else {
        # Non-Windows: Check if running as root
        $isAdmin = $(id -u) -eq 0
    }

    if (-not $Quiet) {
        if ($isAdmin) {
            Write-Verbose "Running with administrator privileges"
        } else {
            Write-Verbose "Running without administrator privileges"
        }
    }

    if ($Required -and -not $isAdmin) {
        throw "This operation requires administrator privileges. Please run PowerShell as Administrator."
    }

    return $isAdmin
}

function Request-AdminRights {
    <#
    .SYNOPSIS
        Requests elevation to administrator privileges.
    .DESCRIPTION
        Restarts the current script or command with elevated privileges.
        On Windows, this triggers a UAC prompt.
    .PARAMETER ScriptPath
        The script to run elevated. If not specified, uses the current script.
    .PARAMETER Arguments
        Additional arguments to pass to the elevated script.
    #>
    [CmdletBinding()]
    param(
        [Parameter()]
        [string]$ScriptPath,

        [Parameter()]
        [string[]]$Arguments
    )

    if (Test-AdminRights -Quiet) {
        Write-Verbose "Already running with administrator privileges"
        return $true
    }

    if (-not $ScriptPath) {
        $ScriptPath = $MyInvocation.PSCommandPath
        if (-not $ScriptPath) {
            Write-Warning "Cannot determine script path for elevation. Please run PowerShell as Administrator manually."
            return $false
        }
    }

    Write-WinMoleStatus -Message "Requesting administrator privileges..." -Type Warning

    try {
        $argList = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$ScriptPath`"")
        if ($Arguments) {
            $argList += $Arguments
        }

        Start-Process -FilePath 'pwsh' -ArgumentList $argList -Verb RunAs -Wait
        return $true
    }
    catch {
        Write-WinMoleStatus -Message "Failed to obtain administrator privileges: $_" -Type Error
        return $false
    }
}

function Invoke-AsAdmin {
    <#
    .SYNOPSIS
        Runs a script block with administrator privileges.
    .DESCRIPTION
        Executes the provided script block in an elevated context.
        If already admin, runs directly. Otherwise, spawns elevated process.
    .PARAMETER ScriptBlock
        The code to execute with admin rights.
    .PARAMETER ArgumentList
        Arguments to pass to the script block.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [scriptblock]$ScriptBlock,

        [Parameter()]
        [object[]]$ArgumentList
    )

    if (Test-AdminRights -Quiet) {
        # Already admin, run directly
        if ($ArgumentList) {
            & $ScriptBlock @ArgumentList
        } else {
            & $ScriptBlock
        }
    } else {
        # Need elevation
        $encodedCommand = [Convert]::ToBase64String(
            [Text.Encoding]::Unicode.GetBytes($ScriptBlock.ToString())
        )

        $argString = if ($ArgumentList) {
            $ArgumentList | ForEach-Object { "`"$_`"" } | Join-String -Separator ' '
        } else { '' }

        try {
            $process = Start-Process -FilePath 'pwsh' -ArgumentList @(
                '-NoProfile',
                '-ExecutionPolicy', 'Bypass',
                '-EncodedCommand', $encodedCommand,
                $argString
            ) -Verb RunAs -PassThru -Wait

            return $process.ExitCode -eq 0
        }
        catch {
            Write-WinMoleStatus -Message "Elevation failed: $_" -Type Error
            return $false
        }
    }
}

function Write-AdminWarning {
    <#
    .SYNOPSIS
        Displays a warning that admin rights are needed.
    #>
    [CmdletBinding()]
    param(
        [Parameter()]
        [string]$Operation = "This operation"
    )

    if (-not (Test-AdminRights -Quiet)) {
        Write-Host ""
        Write-ColorOutput " WARNING " -BackgroundColor Yellow -ForegroundColor Black -NoNewline
        Write-ColorOutput " $Operation requires administrator privileges." -ForegroundColor Yellow
        Write-ColorOutput "          Some features may be limited or unavailable." -ForegroundColor BrightBlack
        Write-Host ""
    }
}
