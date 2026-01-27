function Format-FileSize {
    <#
    .SYNOPSIS
        Formats a file size in bytes to a human-readable string.
    .DESCRIPTION
        Converts bytes to the most appropriate unit (B, KB, MB, GB, TB)
        with configurable precision and formatting options.
    .PARAMETER Bytes
        The size in bytes to format.
    .PARAMETER Precision
        Number of decimal places (default 2).
    .PARAMETER Binary
        Use binary units (1024-based: KiB, MiB, GiB) instead of decimal (1000-based: KB, MB, GB).
    .EXAMPLE
        Format-FileSize 1536
        # Returns: "1.50 KB"
    .EXAMPLE
        Format-FileSize 1073741824
        # Returns: "1.00 GB"
    #>
    [CmdletBinding()]
    [OutputType([string])]
    param(
        [Parameter(Mandatory, Position = 0, ValueFromPipeline)]
        [AllowNull()]
        [long]$Bytes,

        [Parameter()]
        [ValidateRange(0, 5)]
        [int]$Precision = 2,

        [Parameter()]
        [switch]$Binary
    )

    process {
        if ($null -eq $Bytes -or $Bytes -eq 0) {
            return "0 B"
        }

        $negative = $Bytes -lt 0
        $absBytes = [math]::Abs($Bytes)

        if ($Binary) {
            # Binary units (1024-based)
            $units = @('B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB')
            $base = 1024
        }
        else {
            # Decimal units (1000-based, but we use 1024 for compatibility)
            $units = @('B', 'KB', 'MB', 'GB', 'TB', 'PB')
            $base = 1024
        }

        $unitIndex = 0
        $size = [double]$absBytes

        while ($size -ge $base -and $unitIndex -lt ($units.Count - 1)) {
            $size /= $base
            $unitIndex++
        }

        $format = "{0:N$Precision} {1}"
        $result = $format -f $size, $units[$unitIndex]

        if ($negative) {
            return "-$result"
        }

        return $result
    }
}

function ConvertTo-Bytes {
    <#
    .SYNOPSIS
        Converts a human-readable size string to bytes.
    .DESCRIPTION
        Parses strings like "1.5 GB", "500MB", "2TB" and returns the size in bytes.
    .PARAMETER Size
        The size string to parse.
    .EXAMPLE
        ConvertTo-Bytes "1.5 GB"
        # Returns: 1610612736
    #>
    [CmdletBinding()]
    [OutputType([long])]
    param(
        [Parameter(Mandatory, Position = 0, ValueFromPipeline)]
        [string]$Size
    )

    process {
        $Size = $Size.Trim()

        # Parse number and unit
        if ($Size -match '^(-?[\d.]+)\s*(B|KB|KiB|MB|MiB|GB|GiB|TB|TiB|PB|PiB)?$') {
            $number = [double]$Matches[1]
            $unit = if ($Matches[2]) { $Matches[2].ToUpper() } else { 'B' }

            $multiplier = switch ($unit) {
                'B'   { 1 }
                'KB'  { 1024 }
                'KIB' { 1024 }
                'MB'  { 1024 * 1024 }
                'MIB' { 1024 * 1024 }
                'GB'  { 1024 * 1024 * 1024 }
                'GIB' { 1024 * 1024 * 1024 }
                'TB'  { 1024L * 1024 * 1024 * 1024 }
                'TIB' { 1024L * 1024 * 1024 * 1024 }
                'PB'  { 1024L * 1024 * 1024 * 1024 * 1024 }
                'PIB' { 1024L * 1024 * 1024 * 1024 * 1024 }
                default { 1 }
            }

            return [long]($number * $multiplier)
        }

        throw "Invalid size format: $Size"
    }
}

function Compare-FileSize {
    <#
    .SYNOPSIS
        Compares two file sizes and returns the difference.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [long]$Size1,

        [Parameter(Mandatory)]
        [long]$Size2
    )

    $diff = $Size2 - $Size1
    $percentChange = if ($Size1 -ne 0) { [math]::Round(($diff / $Size1) * 100, 1) } else { 100 }

    return @{
        Difference = $diff
        DifferenceFormatted = Format-FileSize $diff
        PercentChange = $percentChange
        Direction = if ($diff -gt 0) { 'Increase' } elseif ($diff -lt 0) { 'Decrease' } else { 'NoChange' }
    }
}

function Get-SizeCategory {
    <#
    .SYNOPSIS
        Returns a category based on size (Tiny, Small, Medium, Large, Huge).
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [long]$Bytes
    )

    if ($Bytes -lt 1KB) { return 'Tiny' }
    elseif ($Bytes -lt 1MB) { return 'Small' }
    elseif ($Bytes -lt 100MB) { return 'Medium' }
    elseif ($Bytes -lt 1GB) { return 'Large' }
    else { return 'Huge' }
}
