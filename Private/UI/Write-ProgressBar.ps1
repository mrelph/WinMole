function Write-ProgressBar {
    <#
    .SYNOPSIS
        Renders a customizable progress bar.
    .DESCRIPTION
        Creates ASCII/Unicode progress bars with percentage, labels, and colors.
    .PARAMETER Percent
        The completion percentage (0-100).
    .PARAMETER Width
        The width of the progress bar in characters.
    .PARAMETER Label
        Optional label to display before the bar.
    .PARAMETER ShowPercent
        Show the percentage value after the bar.
    .PARAMETER FillChar
        Character to use for filled portion.
    .PARAMETER EmptyChar
        Character to use for empty portion.
    .PARAMETER FillColor
        Color for the filled portion.
    .PARAMETER NoNewline
        Don't append a newline.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory, Position = 0)]
        [ValidateRange(0, 100)]
        [double]$Percent,

        [Parameter()]
        [ValidateRange(10, 100)]
        [int]$Width = 20,

        [Parameter()]
        [string]$Label,

        [Parameter()]
        [switch]$ShowPercent,

        [Parameter()]
        [string]$FillChar = [char]0x2588,  # Full block

        [Parameter()]
        [string]$EmptyChar = [char]0x2591,  # Light shade

        [Parameter()]
        [ValidateSet('Green', 'Yellow', 'Red', 'Cyan', 'Blue', 'Magenta', 'White', 'Auto')]
        [string]$FillColor = 'Auto',

        [Parameter()]
        [switch]$NoNewline
    )

    # Determine color based on percentage if Auto
    if ($FillColor -eq 'Auto') {
        $FillColor = switch ($Percent) {
            { $_ -ge 80 } { 'Green' }
            { $_ -ge 50 } { 'Yellow' }
            { $_ -ge 25 } { 'Cyan' }
            default { 'Red' }
        }
    }

    # Calculate filled/empty portions
    $filledWidth = [math]::Round(($Percent / 100) * $Width)
    $emptyWidth = $Width - $filledWidth

    $filled = $FillChar * $filledWidth
    $empty = $EmptyChar * $emptyWidth

    # Build output
    $output = ""

    if ($Label) {
        $output += "$Label "
    }

    $coloredFilled = Get-ColoredString -Text $filled -ForegroundColor $FillColor
    $coloredEmpty = Get-ColoredString -Text $empty -ForegroundColor 'BrightBlack'

    $output += $coloredFilled + $coloredEmpty

    if ($ShowPercent) {
        $percentStr = "{0,5:N1}%" -f $Percent
        $output += " $percentStr"
    }

    if ($NoNewline) {
        Write-Host $output -NoNewline
    } else {
        Write-Host $output
    }
}

function Write-InlineProgressBar {
    <#
    .SYNOPSIS
        Renders a compact inline progress bar for tables and lists.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [double]$Percent,

        [Parameter()]
        [int]$Width = 10,

        [Parameter()]
        [ValidateSet('Green', 'Yellow', 'Red', 'Cyan', 'Auto')]
        [string]$Color = 'Auto'
    )

    if ($Color -eq 'Auto') {
        $Color = switch ($Percent) {
            { $_ -ge 80 } { 'Green' }
            { $_ -ge 50 } { 'Yellow' }
            { $_ -ge 25 } { 'Cyan' }
            default { 'Red' }
        }
    }

    $filledWidth = [math]::Round(($Percent / 100) * $Width)
    $emptyWidth = $Width - $filledWidth

    $filled = Get-ColoredString -Text ([char]0x2588 * $filledWidth) -ForegroundColor $Color
    $empty = Get-ColoredString -Text ([char]0x2591 * $emptyWidth) -ForegroundColor 'BrightBlack'

    return $filled + $empty
}

function Show-OperationProgress {
    <#
    .SYNOPSIS
        Shows operation progress with spinner and message.
    .DESCRIPTION
        Displays a spinner animation with a status message for long-running operations.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [scriptblock]$Operation,

        [Parameter()]
        [string]$Message = 'Processing...',

        [Parameter()]
        [string]$CompletedMessage = 'Done!'
    )

    $spinnerChars = @([char]0x25DC, [char]0x25DD, [char]0x25DE, [char]0x25DF)
    $spinnerIndex = 0

    $cursorVisible = [Console]::CursorVisible
    [Console]::CursorVisible = $false

    try {
        # Start the operation as a job
        $job = Start-Job -ScriptBlock $Operation

        while ($job.State -eq 'Running') {
            $spinner = $spinnerChars[$spinnerIndex % $spinnerChars.Length]
            Write-Host "`r$(Get-ColoredString $spinner -ForegroundColor Cyan) $Message" -NoNewline
            $spinnerIndex++
            Start-Sleep -Milliseconds 100
        }

        # Clear the line
        Write-Host "`r$(' ' * ($Message.Length + 3))" -NoNewline
        Write-Host "`r$(Get-ColoredString ([char]0x2713) -ForegroundColor Green) $CompletedMessage"

        # Get the result
        $result = Receive-Job -Job $job
        Remove-Job -Job $job
        return $result
    }
    finally {
        [Console]::CursorVisible = $cursorVisible
    }
}

function Write-StepProgress {
    <#
    .SYNOPSIS
        Shows step-based progress (e.g., Step 2/5).
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [int]$Current,

        [Parameter(Mandatory)]
        [int]$Total,

        [Parameter(Mandatory)]
        [string]$StepName,

        [Parameter()]
        [ValidateSet('Running', 'Complete', 'Skipped', 'Error')]
        [string]$Status = 'Running'
    )

    $statusIcons = @{
        'Running'  = @{ Icon = [char]0x25B6; Color = 'Cyan' }
        'Complete' = @{ Icon = [char]0x2713; Color = 'Green' }
        'Skipped'  = @{ Icon = [char]0x25CB; Color = 'Yellow' }
        'Error'    = @{ Icon = [char]0x2717; Color = 'Red' }
    }

    $config = $statusIcons[$Status]
    $stepInfo = "[$Current/$Total]"

    $coloredIcon = Get-ColoredString -Text $config.Icon -ForegroundColor $config.Color
    $coloredStep = Get-ColoredString -Text $stepInfo -ForegroundColor 'BrightBlack'

    Write-Host "$coloredIcon $coloredStep $StepName"
}
