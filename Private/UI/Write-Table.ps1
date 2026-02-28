function Write-Table {
    <#
    .SYNOPSIS
        Renders a formatted table with borders and colors.
    .DESCRIPTION
        Creates a styled table output with customizable headers, alignment, and colors.
    .PARAMETER Data
        Array of objects or hashtables to display.
    .PARAMETER Columns
        Column definitions. Each column can specify Name, Width, Align, and Color.
    .PARAMETER Title
        Optional table title.
    .PARAMETER NoHeader
        Skip rendering the header row.
    .PARAMETER Compact
        Use compact borders without box characters.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory, ValueFromPipeline)]
        [object[]]$Data,

        [Parameter()]
        [hashtable[]]$Columns,

        [Parameter()]
        [string]$Title,

        [Parameter()]
        [switch]$NoHeader,

        [Parameter()]
        [switch]$Compact
    )

    begin {
        $allData = @()

        # Box drawing characters
        $box = @{
            TopLeft     = [char]0x250C
            TopRight    = [char]0x2510
            BottomLeft  = [char]0x2514
            BottomRight = [char]0x2518
            Horizontal  = [char]0x2500
            Vertical    = [char]0x2502
            LeftTee     = [char]0x251C
            RightTee    = [char]0x2524
            TopTee      = [char]0x252C
            BottomTee   = [char]0x2534
            Cross       = [char]0x253C
        }
    }

    process {
        $allData += $Data
    }

    end {
        if ($allData.Count -eq 0) { return }

        # Auto-detect columns if not specified
        if (-not $Columns) {
            $sample = $allData[0]
            if ($sample -is [hashtable]) {
                $Columns = $sample.Keys | ForEach-Object { @{ Name = $_; Property = $_ } }
            } else {
                $Columns = $sample.PSObject.Properties | ForEach-Object { @{ Name = $_.Name; Property = $_.Name } }
            }
        }

        # Calculate column widths
        foreach ($col in $Columns) {
            if (-not $col.Width) {
                $maxWidth = $col.Name.Length
                foreach ($row in $allData) {
                    $value = if ($row -is [hashtable]) { $row[$col.Property] } else { $row.($col.Property) }
                    $valueLen = ("$value" -replace '\e\[[0-9;]*m', '').Length
                    if ($valueLen -gt $maxWidth) { $maxWidth = $valueLen }
                }
                $col.Width = [math]::Min($maxWidth + 2, 50)
            }
            if (-not $col.Align) { $col.Align = 'Left' }
        }

        # Helper to pad/align text
        $padText = {
            param($text, $width, $align)
            $cleanText = "$text" -replace '\e\[[0-9;]*m', ''
            $padding = $width - $cleanText.Length
            if ($padding -lt 0) {
                # Truncate
                return $text.Substring(0, $width - 3) + '...'
            }
            switch ($align) {
                'Left'   { return "$text" + (' ' * $padding) }
                'Right'  { return (' ' * $padding) + "$text" }
                'Center' {
                    $left = [math]::Floor($padding / 2)
                    $right = [math]::Ceiling($padding / 2)
                    return (' ' * $left) + "$text" + (' ' * $right)
                }
            }
        }

        # Build borders
        $totalWidth = ($Columns | ForEach-Object { $_.Width }) | Measure-Object -Sum | Select-Object -ExpandProperty Sum
        $totalWidth += ($Columns.Count - 1) * 3 + 4  # separators + padding

        if ($Compact) {
            $topBorder = '-' * $totalWidth
            $midBorder = '-' * $totalWidth
            $bottomBorder = '-' * $totalWidth
            $sep = ' | '
            $leftEdge = '| '
            $rightEdge = ' |'
        } else {
            $topBorder = $box.TopLeft + ($Columns | ForEach-Object { $box.Horizontal * ($_.Width + 2) }) -join $box.TopTee
            $topBorder += $box.TopRight

            $midBorder = $box.LeftTee + ($Columns | ForEach-Object { $box.Horizontal * ($_.Width + 2) }) -join $box.Cross
            $midBorder += $box.RightTee

            $bottomBorder = $box.BottomLeft + ($Columns | ForEach-Object { $box.Horizontal * ($_.Width + 2) }) -join $box.BottomTee
            $bottomBorder += $box.BottomRight

            $sep = " $($box.Vertical) "
            $leftEdge = "$($box.Vertical) "
            $rightEdge = " $($box.Vertical)"
        }

        # Print title if provided
        if ($Title) {
            Write-ColorOutput $Title -ForegroundColor BrightCyan -Style Bold
        }

        # Print top border
        Write-ColorOutput $topBorder -ForegroundColor BrightBlack

        # Print header
        if (-not $NoHeader) {
            $headerCells = $Columns | ForEach-Object {
                $padded = & $padText $_.Name $_.Width $_.Align
                Get-ColoredString -Text $padded -ForegroundColor BrightWhite -Style Bold
            }
            Write-Host "$leftEdge$($headerCells -join $sep)$rightEdge"
            Write-ColorOutput $midBorder -ForegroundColor BrightBlack
        }

        # Print data rows
        foreach ($row in $allData) {
            $cells = $Columns | ForEach-Object {
                $value = if ($row -is [hashtable]) { $row[$_.Property] } else { $row.($_.Property) }
                $padded = & $padText "$value" $_.Width $_.Align

                if ($_.Color) {
                    $color = if ($_.Color -is [scriptblock]) { & $_.Color $value $row } else { $_.Color }
                    Get-ColoredString -Text $padded -ForegroundColor $color
                } else {
                    $padded
                }
            }
            Write-Host "$leftEdge$($cells -join $sep)$rightEdge"
        }

        # Print bottom border
        Write-ColorOutput $bottomBorder -ForegroundColor BrightBlack
    }
}

function Write-KeyValueTable {
    <#
    .SYNOPSIS
        Renders a simple key-value pair table.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [hashtable]$Data,

        [Parameter()]
        [int]$KeyWidth = 20,

        [Parameter()]
        [string]$KeyColor = 'Cyan',

        [Parameter()]
        [string]$ValueColor = 'White'
    )

    $maxKeyLen = ($Data.Keys | ForEach-Object { $_.Length } | Measure-Object -Maximum).Maximum
    $keyWidth = [math]::Max($KeyWidth, $maxKeyLen + 2)

    foreach ($key in $Data.Keys | Sort-Object) {
        $paddedKey = $key.PadRight($keyWidth)
        Write-ColorOutput $paddedKey -ForegroundColor $KeyColor -NoNewline
        Write-ColorOutput ": " -ForegroundColor BrightBlack -NoNewline
        Write-ColorOutput $Data[$key] -ForegroundColor $ValueColor
    }
}

function Write-StatusLine {
    <#
    .SYNOPSIS
        Writes a status line with label, value, and optional bar.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [string]$Label,

        [Parameter(Mandatory)]
        [string]$Value,

        [Parameter()]
        [double]$Percent,

        [Parameter()]
        [int]$LabelWidth = 8,

        [Parameter()]
        [int]$BarWidth = 20,

        [Parameter()]
        [string]$Extra
    )

    $paddedLabel = $Label.PadRight($LabelWidth)
    Write-ColorOutput $paddedLabel -ForegroundColor BrightWhite -NoNewline

    if ($PSBoundParameters.ContainsKey('Percent')) {
        Write-ProgressBar -Percent $Percent -Width $BarWidth -NoNewline
        Write-Host " " -NoNewline
    }

    Write-ColorOutput $Value -ForegroundColor White -NoNewline

    if ($Extra) {
        Write-ColorOutput "  $Extra" -ForegroundColor BrightBlack
    } else {
        Write-Host ""
    }
}
