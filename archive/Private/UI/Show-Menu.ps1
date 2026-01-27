function Show-Menu {
    <#
    .SYNOPSIS
        Displays an interactive menu with keyboard navigation.
    .DESCRIPTION
        Creates a selectable menu with arrow key navigation, highlighting,
        and optional descriptions.
    .PARAMETER Title
        Menu title.
    .PARAMETER Options
        Array of menu options. Can be strings or hashtables with Name, Description, Action.
    .PARAMETER DefaultIndex
        Initially selected index.
    .PARAMETER ReturnIndex
        Return the selected index instead of the option value.
    .PARAMETER Simple
        Use simple numbered input instead of arrow key navigation.
    #>
    [CmdletBinding()]
    param(
        [Parameter()]
        [string]$Title,

        [Parameter(Mandatory)]
        [object[]]$Options,

        [Parameter()]
        [int]$DefaultIndex = 0,

        [Parameter()]
        [switch]$ReturnIndex,

        [Parameter()]
        [switch]$Simple,

        [Parameter()]
        [string]$Prompt = "Use arrow keys to navigate, Enter to select, Q to quit"
    )

    # Normalize options to hashtables
    $menuOptions = @($Options | ForEach-Object {
        if ($_ -is [hashtable]) {
            $_
        } else {
            @{ Name = "$_"; Value = $_ }
        }
    })

    # Use simple mode if console doesn't support ReadKey well
    $useSimple = $Simple
    if (-not $useSimple) {
        try {
            # Test if we can use Console methods
            $null = [Console]::CursorVisible
        } catch {
            $useSimple = $true
        }
    }

    if ($useSimple) {
        # Simple numbered menu
        Clear-Host

        if ($Title) {
            Write-WinMoleHeader -Title $Title -Style Simple
            Write-Host ""
        }

        for ($i = 0; $i -lt $menuOptions.Count; $i++) {
            $option = $menuOptions[$i]
            $num = $i + 1
            Write-Host "  [$num] $($option.Name)"
            if ($option.Description) {
                Write-ColorOutput "      $($option.Description)" -ForegroundColor BrightBlack
            }
        }

        Write-Host ""
        Write-Host "  [0] Cancel / Go Back"
        Write-Host ""

        $choice = Read-Host "Enter choice (1-$($menuOptions.Count))"

        if ($choice -eq '0' -or $choice -eq 'q' -or $choice -eq 'Q' -or [string]::IsNullOrWhiteSpace($choice)) {
            return $null
        }

        $choiceNum = 0
        if ([int]::TryParse($choice, [ref]$choiceNum) -and $choiceNum -ge 1 -and $choiceNum -le $menuOptions.Count) {
            $selected = $menuOptions[$choiceNum - 1]

            if ($ReturnIndex) {
                return $choiceNum - 1
            }

            if ($selected.Action) {
                & $selected.Action
            }

            if ($selected.Value) {
                return $selected.Value
            } else {
                return $selected.Name
            }
        }

        return $null
    }

    # Interactive arrow-key menu
    $selectedIndex = $DefaultIndex
    $cursorVisible = $true
    try { $cursorVisible = [Console]::CursorVisible } catch { }

    try {
        [Console]::CursorVisible = $false
    } catch { }

    try {
        while ($true) {
            # Clear and redraw
            Clear-Host

            if ($Title) {
                Write-WinMoleHeader -Title $Title -Style Simple
                Write-Host ""
            }

            # Draw options
            for ($i = 0; $i -lt $menuOptions.Count; $i++) {
                $option = $menuOptions[$i]
                $isSelected = ($i -eq $selectedIndex)

                $prefix = if ($isSelected) {
                    Get-ColoredString -Text " > " -ForegroundColor Cyan -Style Bold
                } else {
                    "   "
                }

                $name = if ($isSelected) {
                    Get-ColoredString -Text $option.Name -ForegroundColor BrightCyan -Style Bold
                } else {
                    Get-ColoredString -Text $option.Name -ForegroundColor White
                }

                Write-Host "$prefix$name"

                if ($option.Description -and $isSelected) {
                    $desc = Get-ColoredString -Text "     $($option.Description)" -ForegroundColor BrightBlack
                    Write-Host $desc
                }
            }

            Write-Host ""
            Write-ColorOutput $Prompt -ForegroundColor BrightBlack

            # Read key
            $key = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')

            switch ($key.VirtualKeyCode) {
                38 { # UpArrow
                    $selectedIndex = if ($selectedIndex -le 0) { $menuOptions.Count - 1 } else { $selectedIndex - 1 }
                }
                40 { # DownArrow
                    $selectedIndex = if ($selectedIndex -ge ($menuOptions.Count - 1)) { 0 } else { $selectedIndex + 1 }
                }
                13 { # Enter
                    try { [Console]::CursorVisible = $cursorVisible } catch { }
                    Clear-Host

                    if ($ReturnIndex) {
                        return $selectedIndex
                    }

                    $selected = $menuOptions[$selectedIndex]
                    if ($selected.Action) {
                        & $selected.Action
                    }

                    if ($selected.Value) {
                        return $selected.Value
                    } else {
                        return $selected.Name
                    }
                }
                81 { # Q
                    try { [Console]::CursorVisible = $cursorVisible } catch { }
                    Clear-Host
                    return $null
                }
                27 { # Escape
                    try { [Console]::CursorVisible = $cursorVisible } catch { }
                    Clear-Host
                    return $null
                }
            }
        }
    }
    catch {
        # If interactive mode fails, fall back to simple mode
        try { [Console]::CursorVisible = $cursorVisible } catch { }
        return Show-Menu -Title $Title -Options $Options -DefaultIndex $DefaultIndex -ReturnIndex:$ReturnIndex -Simple
    }
    finally {
        try { [Console]::CursorVisible = $cursorVisible } catch { }
    }
}

function Show-Confirmation {
    <#
    .SYNOPSIS
        Shows a Y/N confirmation prompt.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [string]$Message,

        [Parameter()]
        [bool]$Default = $false
    )

    $defaultHint = if ($Default) { "[Y/n]" } else { "[y/N]" }
    Write-ColorOutput "$Message $defaultHint " -ForegroundColor Yellow -NoNewline

    $response = Read-Host

    if ([string]::IsNullOrWhiteSpace($response)) {
        return $Default
    }

    return $response.ToLower() -in @('y', 'yes')
}

function Show-MultiSelect {
    <#
    .SYNOPSIS
        Shows a multi-select menu with checkboxes.
    #>
    [CmdletBinding()]
    param(
        [Parameter()]
        [string]$Title,

        [Parameter(Mandatory)]
        [object[]]$Options,

        [Parameter()]
        [int[]]$DefaultSelected = @(),

        [Parameter()]
        [string]$Prompt = "Space to toggle, Enter to confirm, A to select all, Q to quit"
    )

    # Normalize options
    $menuOptions = $Options | ForEach-Object {
        if ($_ -is [hashtable]) {
            @{ Name = $_.Name; Value = if ($_.Value) { $_.Value } else { $_.Name }; Selected = $false }
        } else {
            @{ Name = "$_"; Value = $_; Selected = $false }
        }
    }

    # Apply defaults
    foreach ($idx in $DefaultSelected) {
        if ($idx -ge 0 -and $idx -lt $menuOptions.Count) {
            $menuOptions[$idx].Selected = $true
        }
    }

    $selectedIndex = 0
    $cursorVisible = [Console]::CursorVisible
    [Console]::CursorVisible = $false

    try {
        while ($true) {
            Clear-Host

            if ($Title) {
                Write-WinMoleHeader -Title $Title -Style Simple
                Write-Host ""
            }

            for ($i = 0; $i -lt $menuOptions.Count; $i++) {
                $option = $menuOptions[$i]
                $isSelected = ($i -eq $selectedIndex)

                $checkbox = if ($option.Selected) {
                    Get-ColoredString -Text "[X]" -ForegroundColor Green
                } else {
                    Get-ColoredString -Text "[ ]" -ForegroundColor BrightBlack
                }

                $prefix = if ($isSelected) {
                    Get-ColoredString -Text ">" -ForegroundColor Cyan
                } else {
                    " "
                }

                $name = if ($isSelected) {
                    Get-ColoredString -Text $option.Name -ForegroundColor BrightCyan
                } else {
                    $option.Name
                }

                Write-Host "$prefix $checkbox $name"
            }

            Write-Host ""
            Write-ColorOutput $Prompt -ForegroundColor BrightBlack

            $key = [Console]::ReadKey($true)

            switch ($key.Key) {
                'UpArrow' {
                    $selectedIndex = if ($selectedIndex -le 0) { $menuOptions.Count - 1 } else { $selectedIndex - 1 }
                }
                'DownArrow' {
                    $selectedIndex = if ($selectedIndex -ge ($menuOptions.Count - 1)) { 0 } else { $selectedIndex + 1 }
                }
                'Spacebar' {
                    $menuOptions[$selectedIndex].Selected = -not $menuOptions[$selectedIndex].Selected
                }
                'A' {
                    $allSelected = ($menuOptions | Where-Object { $_.Selected }).Count -eq $menuOptions.Count
                    $menuOptions | ForEach-Object { $_.Selected = -not $allSelected }
                }
                'Enter' {
                    [Console]::CursorVisible = $cursorVisible
                    Clear-Host
                    return $menuOptions | Where-Object { $_.Selected } | ForEach-Object { $_.Value }
                }
                'Q' {
                    [Console]::CursorVisible = $cursorVisible
                    Clear-Host
                    return @()
                }
                'Escape' {
                    [Console]::CursorVisible = $cursorVisible
                    Clear-Host
                    return @()
                }
            }
        }
    }
    finally {
        [Console]::CursorVisible = $cursorVisible
    }
}
