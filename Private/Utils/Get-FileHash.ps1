function Get-WinMoleFileHash {
    <#
    .SYNOPSIS
        Calculates file hash with optimized performance for large files.
    .DESCRIPTION
        Wrapper around Get-FileHash with additional features like progress
        reporting for large files and caching.
    .PARAMETER Path
        The file path to hash.
    .PARAMETER Algorithm
        Hash algorithm to use (default MD5 for speed, SHA256 for security).
    .PARAMETER QuickHash
        Only hash the first 64KB for quick comparison (not cryptographically secure).
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory, Position = 0, ValueFromPipeline, ValueFromPipelineByPropertyName)]
        [Alias('FullName')]
        [string]$Path,

        [Parameter()]
        [ValidateSet('MD5', 'SHA1', 'SHA256', 'SHA384', 'SHA512')]
        [string]$Algorithm = 'MD5',

        [Parameter()]
        [switch]$QuickHash
    )

    process {
        if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
            Write-Warning "File not found: $Path"
            return $null
        }

        try {
            if ($QuickHash) {
                # Quick hash: only read first 64KB
                $bytes = [System.IO.File]::ReadAllBytes($Path) | Select-Object -First 65536

                $hashAlgo = switch ($Algorithm) {
                    'MD5'    { [System.Security.Cryptography.MD5]::Create() }
                    'SHA1'   { [System.Security.Cryptography.SHA1]::Create() }
                    'SHA256' { [System.Security.Cryptography.SHA256]::Create() }
                    'SHA384' { [System.Security.Cryptography.SHA384]::Create() }
                    'SHA512' { [System.Security.Cryptography.SHA512]::Create() }
                }

                $hashBytes = $hashAlgo.ComputeHash($bytes)
                $hashString = [BitConverter]::ToString($hashBytes) -replace '-', ''

                return [PSCustomObject]@{
                    Algorithm = $Algorithm
                    Hash = $hashString
                    Path = $Path
                    QuickHash = $true
                }
            }
            else {
                $result = Get-FileHash -LiteralPath $Path -Algorithm $Algorithm -ErrorAction Stop
                return [PSCustomObject]@{
                    Algorithm = $result.Algorithm
                    Hash = $result.Hash
                    Path = $result.Path
                    QuickHash = $false
                }
            }
        }
        catch {
            Write-Warning "Failed to hash file: $Path - $_"
            return $null
        }
    }
}

function Find-DuplicateFiles {
    <#
    .SYNOPSIS
        Finds duplicate files in a directory based on hash.
    .DESCRIPTION
        Scans a directory for files with identical content using a two-phase approach:
        1. Group by size (quick filter)
        2. Hash files with same size to find true duplicates
    .PARAMETER Path
        The directory to scan.
    .PARAMETER MinSize
        Minimum file size to consider (default 1KB).
    .PARAMETER Recurse
        Search subdirectories.
    .PARAMETER Include
        File patterns to include.
    .PARAMETER Exclude
        File patterns to exclude.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory, Position = 0)]
        [string]$Path,

        [Parameter()]
        [long]$MinSize = 1KB,

        [Parameter()]
        [switch]$Recurse,

        [Parameter()]
        [string[]]$Include,

        [Parameter()]
        [string[]]$Exclude
    )

    if (-not (Test-Path $Path -PathType Container)) {
        Write-Error "Directory not found: $Path"
        return
    }

    Write-WinMoleStatus -Message "Scanning for files..." -Type Progress

    # Get all files
    $getChildParams = @{
        Path = $Path
        File = $true
        ErrorAction = 'SilentlyContinue'
    }

    if ($Recurse) { $getChildParams.Recurse = $true }
    if ($Include) { $getChildParams.Include = $Include }
    if ($Exclude) { $getChildParams.Exclude = $Exclude }

    $files = Get-ChildItem @getChildParams | Where-Object { $_.Length -ge $MinSize }

    Write-WinMoleStatus -Message "Found $($files.Count) files, grouping by size..." -Type Progress

    # Phase 1: Group by size
    $sizeGroups = $files | Group-Object Length | Where-Object { $_.Count -gt 1 }

    $potentialDuplicates = ($sizeGroups | Measure-Object -Property Count -Sum).Sum
    Write-WinMoleStatus -Message "$potentialDuplicates files in $($sizeGroups.Count) size groups, hashing..." -Type Progress

    # Phase 2: Hash files with same size
    $duplicates = @()
    $totalGroups = $sizeGroups.Count
    $currentGroup = 0

    foreach ($group in $sizeGroups) {
        $currentGroup++
        Write-Progress -Activity "Finding duplicates" -Status "Processing size group $currentGroup of $totalGroups" `
            -PercentComplete (($currentGroup / $totalGroups) * 100)

        $hashes = @{}

        foreach ($file in $group.Group) {
            $hash = Get-WinMoleFileHash -Path $file.FullName -QuickHash

            if ($hash) {
                if ($hashes.ContainsKey($hash.Hash)) {
                    $hashes[$hash.Hash] += $file
                }
                else {
                    $hashes[$hash.Hash] = @($file)
                }
            }
        }

        # Find groups with duplicates
        foreach ($hashGroup in $hashes.GetEnumerator() | Where-Object { $_.Value.Count -gt 1 }) {
            # Verify with full hash for certainty
            $fullHashes = @{}

            foreach ($file in $hashGroup.Value) {
                $fullHash = Get-WinMoleFileHash -Path $file.FullName

                if ($fullHash) {
                    if ($fullHashes.ContainsKey($fullHash.Hash)) {
                        $fullHashes[$fullHash.Hash] += $file
                    }
                    else {
                        $fullHashes[$fullHash.Hash] = @($file)
                    }
                }
            }

            foreach ($dupGroup in $fullHashes.GetEnumerator() | Where-Object { $_.Value.Count -gt 1 }) {
                $duplicates += [PSCustomObject]@{
                    Hash = $dupGroup.Key
                    Size = $dupGroup.Value[0].Length
                    SizeFormatted = Format-FileSize $dupGroup.Value[0].Length
                    Count = $dupGroup.Value.Count
                    WastedSpace = ($dupGroup.Value.Count - 1) * $dupGroup.Value[0].Length
                    WastedSpaceFormatted = Format-FileSize (($dupGroup.Value.Count - 1) * $dupGroup.Value[0].Length)
                    Files = $dupGroup.Value.FullName
                }
            }
        }
    }

    Write-Progress -Activity "Finding duplicates" -Completed

    $totalWasted = ($duplicates | Measure-Object -Property WastedSpace -Sum).Sum

    return @{
        Duplicates = $duplicates
        TotalGroups = $duplicates.Count
        TotalWastedSpace = $totalWasted
        TotalWastedSpaceFormatted = Format-FileSize $totalWasted
    }
}
