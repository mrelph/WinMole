function Write-ColorOutput {
    <#
    .SYNOPSIS
        Writes colored output to the console using ANSI escape codes.
    .DESCRIPTION
        Provides a consistent way to write colored text with support for
        foreground colors, background colors, and text styles.
    .PARAMETER Text
        The text to display.
    .PARAMETER ForegroundColor
        The foreground (text) color.
    .PARAMETER BackgroundColor
        The background color.
    .PARAMETER Style
        Text style (Bold, Dim, Italic, Underline).
    .PARAMETER NoNewline
        Don't append a newline at the end.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory, Position = 0, ValueFromPipeline)]
        [AllowEmptyString()]
        [string]$Text,

        [Parameter()]
        [ValidateSet('Black', 'Red', 'Green', 'Yellow', 'Blue', 'Magenta', 'Cyan', 'White',
                     'BrightBlack', 'BrightRed', 'BrightGreen', 'BrightYellow',
                     'BrightBlue', 'BrightMagenta', 'BrightCyan', 'BrightWhite', 'Default')]
        [string]$ForegroundColor = 'Default',

        [Parameter()]
        [ValidateSet('Black', 'Red', 'Green', 'Yellow', 'Blue', 'Magenta', 'Cyan', 'White', 'Default')]
        [string]$BackgroundColor = 'Default',

        [Parameter()]
        [ValidateSet('Normal', 'Bold', 'Dim', 'Italic', 'Underline')]
        [string]$Style = 'Normal',

        [Parameter()]
        [switch]$NoNewline
    )

    begin {
        # ANSI escape code definitions
        $ESC = [char]0x1B
        $Reset = "$ESC[0m"

        # Foreground color codes
        $FGColors = @{
            'Black'         = '30'
            'Red'           = '31'
            'Green'         = '32'
            'Yellow'        = '33'
            'Blue'          = '34'
            'Magenta'       = '35'
            'Cyan'          = '36'
            'White'         = '37'
            'BrightBlack'   = '90'
            'BrightRed'     = '91'
            'BrightGreen'   = '92'
            'BrightYellow'  = '93'
            'BrightBlue'    = '94'
            'BrightMagenta' = '95'
            'BrightCyan'    = '96'
            'BrightWhite'   = '97'
            'Default'       = '39'
        }

        # Background color codes
        $BGColors = @{
            'Black'   = '40'
            'Red'     = '41'
            'Green'   = '42'
            'Yellow'  = '43'
            'Blue'    = '44'
            'Magenta' = '45'
            'Cyan'    = '46'
            'White'   = '47'
            'Default' = '49'
        }

        # Style codes
        $Styles = @{
            'Normal'    = '0'
            'Bold'      = '1'
            'Dim'       = '2'
            'Italic'    = '3'
            'Underline' = '4'
        }
    }

    process {
        if (-not $script:SupportsAnsi) {
            # Fall back to plain text if ANSI not supported
            if ($NoNewline) {
                Write-Host $Text -NoNewline
            } else {
                Write-Host $Text
            }
            return
        }

        # Build ANSI sequence
        $codes = @()

        if ($Style -ne 'Normal') {
            $codes += $Styles[$Style]
        }

        if ($ForegroundColor -ne 'Default') {
            $codes += $FGColors[$ForegroundColor]
        }

        if ($BackgroundColor -ne 'Default') {
            $codes += $BGColors[$BackgroundColor]
        }

        if ($codes.Count -gt 0) {
            $sequence = "$ESC[$($codes -join ';')m"
            $output = "${sequence}${Text}${Reset}"
        } else {
            $output = $Text
        }

        if ($NoNewline) {
            Write-Host $output -NoNewline
        } else {
            Write-Host $output
        }
    }
}

function Get-ColoredString {
    <#
    .SYNOPSIS
        Returns a colored string without writing to console.
    .DESCRIPTION
        Creates an ANSI-colored string that can be used in composite output.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory, Position = 0)]
        [AllowEmptyString()]
        [string]$Text,

        [Parameter()]
        [ValidateSet('Black', 'Red', 'Green', 'Yellow', 'Blue', 'Magenta', 'Cyan', 'White',
                     'BrightBlack', 'BrightRed', 'BrightGreen', 'BrightYellow',
                     'BrightBlue', 'BrightMagenta', 'BrightCyan', 'BrightWhite', 'Default')]
        [string]$ForegroundColor = 'Default',

        [Parameter()]
        [ValidateSet('Normal', 'Bold', 'Dim', 'Italic', 'Underline')]
        [string]$Style = 'Normal'
    )

    if (-not $script:SupportsAnsi) {
        return $Text
    }

    $ESC = [char]0x1B
    $Reset = "$ESC[0m"

    $FGColors = @{
        'Black'         = '30'; 'Red'           = '31'; 'Green'         = '32'
        'Yellow'        = '33'; 'Blue'          = '34'; 'Magenta'       = '35'
        'Cyan'          = '36'; 'White'         = '37'; 'BrightBlack'   = '90'
        'BrightRed'     = '91'; 'BrightGreen'   = '92'; 'BrightYellow'  = '93'
        'BrightBlue'    = '94'; 'BrightMagenta' = '95'; 'BrightCyan'    = '96'
        'BrightWhite'   = '97'; 'Default'       = '39'
    }

    $Styles = @{
        'Normal' = '0'; 'Bold' = '1'; 'Dim' = '2'; 'Italic' = '3'; 'Underline' = '4'
    }

    $codes = @()
    if ($Style -ne 'Normal') { $codes += $Styles[$Style] }
    if ($ForegroundColor -ne 'Default') { $codes += $FGColors[$ForegroundColor] }

    if ($codes.Count -gt 0) {
        return "$ESC[$($codes -join ';')m${Text}${Reset}"
    }
    return $Text
}

function Write-WinMoleHeader {
    <#
    .SYNOPSIS
        Writes a styled WinMole header/title.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [string]$Title,

        [Parameter()]
        [ValidateSet('Box', 'Line', 'Simple')]
        [string]$Style = 'Box'
    )

    $ESC = [char]0x1B
    $width = 52

    switch ($Style) {
        'Box' {
            $topBorder = [string][char]0x2554 + ([string][char]0x2550 * ($width - 2)) + [string][char]0x2557
            $bottomBorder = [string][char]0x255A + ([string][char]0x2550 * ($width - 2)) + [string][char]0x255D
            $side = [string][char]0x2551

            $paddedTitle = $Title.PadLeft(([math]::Floor(($width - 2 + $Title.Length) / 2))).PadRight($width - 2)

            Write-ColorOutput $topBorder -ForegroundColor Cyan
            Write-ColorOutput "$side$paddedTitle$side" -ForegroundColor Cyan
            Write-ColorOutput $bottomBorder -ForegroundColor Cyan
        }
        'Line' {
            $line = '=' * $width
            Write-ColorOutput $line -ForegroundColor Cyan
            Write-ColorOutput $Title -ForegroundColor BrightCyan -Style Bold
            Write-ColorOutput $line -ForegroundColor Cyan
        }
        'Simple' {
            Write-ColorOutput "=== $Title ===" -ForegroundColor BrightCyan -Style Bold
        }
    }
}

function Write-WinMoleSection {
    <#
    .SYNOPSIS
        Writes a section separator with title.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [string]$Title
    )

    $width = 52
    $separator = [string][char]0x2550 * ($width - 2)
    $side = [string][char]0x2560
    $endSide = [string][char]0x2563

    Write-Host ""
    Write-ColorOutput "$side$separator$endSide" -ForegroundColor Cyan
    Write-ColorOutput "  $Title" -ForegroundColor BrightWhite -Style Bold
}

function Write-WinMoleStatus {
    <#
    .SYNOPSIS
        Writes a status message with icon.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [string]$Message,

        [Parameter()]
        [ValidateSet('Info', 'Success', 'Warning', 'Error', 'Progress')]
        [string]$Type = 'Info'
    )

    $icons = @{
        'Info'     = @{ Icon = [char]0x2139; Color = 'Cyan' }
        'Success'  = @{ Icon = [char]0x2713; Color = 'Green' }
        'Warning'  = @{ Icon = [char]0x26A0; Color = 'Yellow' }
        'Error'    = @{ Icon = [char]0x2717; Color = 'Red' }
        'Progress' = @{ Icon = [char]0x25B6; Color = 'Blue' }
    }

    $config = $icons[$Type]
    Write-ColorOutput "$($config.Icon) " -ForegroundColor $config.Color -NoNewline
    Write-ColorOutput $Message -ForegroundColor White
}
