function Write-Tree {
    <#
    .SYNOPSIS
        Renders a tree visualization of directories or hierarchical data.
    .DESCRIPTION
        Creates a visual tree representation with size information,
        progress bars, and color-coded items.
    .PARAMETER Path
        The root path to display.
    .PARAMETER Data
        Hierarchical data structure to display.
    .PARAMETER Depth
        Maximum depth to display.
    .PARAMETER ShowSize
        Show size information for files/folders.
    .PARAMETER ShowPercent
        Show percentage bars.
    .PARAMETER TopN
        Only show the top N items by size at each level.
    #>
    [CmdletBinding(DefaultParameterSetName = 'Path')]
    param(
        [Parameter(Mandatory, ParameterSetName = 'Path', Position = 0)]
        [string]$Path,

        [Parameter(Mandatory, ParameterSetName = 'Data')]
        [object]$Data,

        [Parameter()]
        [int]$Depth = 3,

        [Parameter()]
        [switch]$ShowSize,

        [Parameter()]
        [switch]$ShowPercent,

        [Parameter()]
        [int]$TopN = 10,

        [Parameter()]
        [long]$TotalSize = 0
    )

    # Tree drawing characters
    $tree = @{
        Branch    = [char]0x251C  # ├
        Last      = [char]0x2514  # └
        Vertical  = [char]0x2502  # │
        Horizontal = [char]0x2500 # ─
        Folder    = [char]0x1F4C1 # Folder emoji fallback to text
        File      = [char]0x1F4C4 # File emoji fallback to text
    }

    # Use simpler characters for wider compatibility
    $folderIcon = Get-ColoredString -Text $([char]0x25A0) -ForegroundColor Yellow
    $fileIcon = Get-ColoredString -Text $([char]0x25AA) -ForegroundColor BrightBlack

    function Get-FolderSizeRecursive {
        param([string]$FolderPath, [int]$MaxDepth, [int]$CurrentDepth = 0)

        $result = @{
            Name = Split-Path $FolderPath -Leaf
            Path = $FolderPath
            Size = 0
            Children = @()
            IsDirectory = $true
        }

        if ($CurrentDepth -ge $MaxDepth) {
            # Just calculate total size without children
            try {
                $result.Size = (Get-ChildItem -Path $FolderPath -Recurse -File -Force -ErrorAction SilentlyContinue |
                    Measure-Object -Property Length -Sum -ErrorAction SilentlyContinue).Sum
            } catch {
                $result.Size = 0
            }
            return $result
        }

        try {
            $items = Get-ChildItem -Path $FolderPath -Force -ErrorAction SilentlyContinue

            foreach ($item in $items) {
                if ($item.PSIsContainer) {
                    $child = Get-FolderSizeRecursive -FolderPath $item.FullName -MaxDepth $MaxDepth -CurrentDepth ($CurrentDepth + 1)
                    $result.Children += $child
                    $result.Size += $child.Size
                } else {
                    $result.Size += $item.Length
                }
            }
        } catch {
            # Access denied or other error
        }

        return $result
    }

    function Write-TreeNode {
        param(
            [object]$Node,
            [string]$Prefix = '',
            [bool]$IsLast = $true,
            [long]$ParentSize = 0,
            [int]$CurrentDepth = 0,
            [int]$MaxDepth
        )

        # Determine branch character
        $branchChar = if ($IsLast) { $tree.Last } else { $tree.Branch }
        $connector = "$branchChar$($tree.Horizontal)$($tree.Horizontal) "

        # Icon
        $icon = if ($Node.IsDirectory) { $folderIcon } else { $fileIcon }

        # Size string
        $sizeStr = ''
        if ($ShowSize -and $Node.Size) {
            $sizeStr = " ($(Format-FileSize $Node.Size))"
        }

        # Percentage bar
        $percentBar = ''
        if ($ShowPercent -and $ParentSize -gt 0 -and $Node.Size) {
            $percent = [math]::Round(($Node.Size / $ParentSize) * 100, 1)
            $percentBar = " " + (Write-InlineProgressBar -Percent $percent -Width 10)
            $percentBar += " $percent%"
        }

        # Color based on size
        $nameColor = if ($Node.IsDirectory) { 'BrightCyan' } else { 'White' }

        # Output the node
        $coloredPrefix = Get-ColoredString -Text $Prefix -ForegroundColor BrightBlack
        $coloredBranch = Get-ColoredString -Text $connector -ForegroundColor BrightBlack
        $coloredName = Get-ColoredString -Text $Node.Name -ForegroundColor $nameColor
        $coloredSize = Get-ColoredString -Text $sizeStr -ForegroundColor BrightBlack

        Write-Host "$coloredPrefix$coloredBranch$icon $coloredName$coloredSize$percentBar"

        # Process children
        if ($Node.Children -and $CurrentDepth -lt $MaxDepth) {
            $sortedChildren = $Node.Children | Sort-Object Size -Descending | Select-Object -First $TopN
            $childCount = $sortedChildren.Count

            for ($i = 0; $i -lt $childCount; $i++) {
                $child = $sortedChildren[$i]
                $isChildLast = ($i -eq ($childCount - 1))
                $newPrefix = $Prefix + $(if ($IsLast) { '    ' } else { "$($tree.Vertical)   " })

                Write-TreeNode -Node $child -Prefix $newPrefix -IsLast $isChildLast `
                    -ParentSize $Node.Size -CurrentDepth ($CurrentDepth + 1) -MaxDepth $MaxDepth
            }

            # Show if items were truncated
            $totalChildren = $Node.Children.Count
            if ($totalChildren -gt $TopN) {
                $remaining = $totalChildren - $TopN
                $newPrefix = $Prefix + $(if ($IsLast) { '    ' } else { "$($tree.Vertical)   " })
                $truncMsg = Get-ColoredString -Text "... and $remaining more items" -ForegroundColor BrightBlack
                Write-Host "$newPrefix    $truncMsg"
            }
        }
    }

    if ($PSCmdlet.ParameterSetName -eq 'Path') {
        if (-not (Test-Path $Path)) {
            Write-WinMoleStatus -Message "Path not found: $Path" -Type Error
            return
        }

        # Get drive info for root
        $driveInfo = Get-PSDrive -Name (Split-Path $Path -Qualifier).TrimEnd(':')
        $driveFree = $driveInfo.Free
        $driveUsed = $driveInfo.Used
        $driveTotal = $driveFree + $driveUsed

        # Print header
        $rootName = if ($Path -match '^[A-Z]:\\?$') { $Path } else { Split-Path $Path -Leaf }
        $headerInfo = "$rootName ($(Format-FileSize $driveTotal) total, $(Format-FileSize $driveFree) free)"
        Write-ColorOutput $folderIcon -NoNewline
        Write-ColorOutput " $headerInfo" -ForegroundColor BrightCyan -Style Bold

        # Calculate sizes
        Write-WinMoleStatus -Message "Calculating folder sizes..." -Type Progress
        $rootNode = Get-FolderSizeRecursive -FolderPath $Path -MaxDepth $Depth

        if ($TotalSize -eq 0) { $TotalSize = $driveUsed }

        # Sort and display top items
        if ($rootNode.Children) {
            $sortedChildren = $rootNode.Children | Sort-Object Size -Descending | Select-Object -First $TopN
            $childCount = $sortedChildren.Count

            for ($i = 0; $i -lt $childCount; $i++) {
                $child = $sortedChildren[$i]
                $isLast = ($i -eq ($childCount - 1))
                Write-TreeNode -Node $child -Prefix '' -IsLast $isLast `
                    -ParentSize $TotalSize -CurrentDepth 0 -MaxDepth $Depth
            }
        }
    } else {
        # Use provided data structure
        Write-TreeNode -Node $Data -Prefix '' -IsLast $true `
            -ParentSize $TotalSize -CurrentDepth 0 -MaxDepth $Depth
    }
}

function Write-SimpleTree {
    <#
    .SYNOPSIS
        Renders a simple list as a tree.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [string[]]$Items,

        [Parameter()]
        [string]$Title,

        [Parameter()]
        [string]$Icon = [char]0x25CF
    )

    if ($Title) {
        Write-ColorOutput $Title -ForegroundColor BrightCyan -Style Bold
    }

    $tree = @{
        Branch = [char]0x251C
        Last   = [char]0x2514
        Line   = [char]0x2500
    }

    for ($i = 0; $i -lt $Items.Count; $i++) {
        $isLast = ($i -eq ($Items.Count - 1))
        $branch = if ($isLast) { $tree.Last } else { $tree.Branch }

        $branchStr = Get-ColoredString -Text "$branch$($tree.Line) " -ForegroundColor BrightBlack
        $iconStr = Get-ColoredString -Text "$Icon " -ForegroundColor Cyan

        Write-Host "$branchStr$iconStr$($Items[$i])"
    }
}
