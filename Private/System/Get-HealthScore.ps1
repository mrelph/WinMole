function Get-HealthScore {
    <#
    .SYNOPSIS
        Calculates an overall system health score.
    .DESCRIPTION
        Analyzes various system metrics and returns a health score from 0-100
        along with individual component scores and recommendations.
    .PARAMETER Detailed
        Return detailed breakdown of scores.
    #>
    [CmdletBinding()]
    param(
        [Parameter()]
        [switch]$Detailed
    )

    $scores = @{}
    $recommendations = @()
    $maxScore = 100

    # CPU Score (0-20 points)
    # Lower CPU usage = higher score
    try {
        $cpuUsage = (Get-CimInstance -ClassName Win32_Processor -ErrorAction SilentlyContinue |
            Measure-Object -Property LoadPercentage -Average).Average

        if ($null -eq $cpuUsage) { $cpuUsage = 0 }

        $cpuScore = [math]::Max(0, 20 - ($cpuUsage / 5))
        $scores['CPU'] = @{
            Score = [math]::Round($cpuScore, 1)
            MaxScore = 20
            Value = "$([math]::Round($cpuUsage))% usage"
            Status = switch ($cpuUsage) {
                { $_ -lt 30 } { 'Good' }
                { $_ -lt 70 } { 'Fair' }
                default { 'High' }
            }
        }

        if ($cpuUsage -gt 80) {
            $recommendations += "High CPU usage detected. Check Task Manager for resource-intensive processes."
        }
    }
    catch {
        $scores['CPU'] = @{ Score = 10; MaxScore = 20; Value = 'Unknown'; Status = 'Unknown' }
    }

    # Memory Score (0-25 points)
    # More available memory = higher score
    try {
        $os = Get-CimInstance -ClassName Win32_OperatingSystem -ErrorAction SilentlyContinue
        $totalMemGB = [math]::Round($os.TotalVisibleMemorySize / 1MB, 1)
        $freeMemGB = [math]::Round($os.FreePhysicalMemory / 1MB, 1)
        $usedPercent = [math]::Round((($os.TotalVisibleMemorySize - $os.FreePhysicalMemory) / $os.TotalVisibleMemorySize) * 100)

        $memScore = [math]::Max(0, 25 - ($usedPercent / 4))
        $scores['Memory'] = @{
            Score = [math]::Round($memScore, 1)
            MaxScore = 25
            Value = "$usedPercent% used ($([math]::Round($totalMemGB - $freeMemGB, 1))/$totalMemGB GB)"
            Status = switch ($usedPercent) {
                { $_ -lt 60 } { 'Good' }
                { $_ -lt 85 } { 'Fair' }
                default { 'High' }
            }
        }

        if ($usedPercent -gt 90) {
            $recommendations += "Memory usage is very high. Consider closing unused applications or adding more RAM."
        }
    }
    catch {
        $scores['Memory'] = @{ Score = 12; MaxScore = 25; Value = 'Unknown'; Status = 'Unknown' }
    }

    # Disk Score (0-25 points)
    # Based on free space on system drive
    try {
        $systemDrive = Get-PSDrive -Name C -ErrorAction SilentlyContinue
        if ($systemDrive) {
            $totalGB = [math]::Round(($systemDrive.Used + $systemDrive.Free) / 1GB, 1)
            $freeGB = [math]::Round($systemDrive.Free / 1GB, 1)
            $freePercent = [math]::Round(($systemDrive.Free / ($systemDrive.Used + $systemDrive.Free)) * 100)

            $diskScore = [math]::Min(25, $freePercent / 4)
            $scores['Disk'] = @{
                Score = [math]::Round($diskScore, 1)
                MaxScore = 25
                Value = "$freeGB GB free ($freePercent%)"
                Status = switch ($freePercent) {
                    { $_ -gt 25 } { 'Good' }
                    { $_ -gt 10 } { 'Fair' }
                    default { 'Low' }
                }
            }

            if ($freePercent -lt 10) {
                $recommendations += "System drive is nearly full. Run cleanup to reclaim disk space."
            }
            elseif ($freePercent -lt 20) {
                $recommendations += "System drive has limited free space. Consider cleaning up temporary files."
            }
        }
    }
    catch {
        $scores['Disk'] = @{ Score = 12; MaxScore = 25; Value = 'Unknown'; Status = 'Unknown' }
    }

    # Startup Score (0-15 points)
    # Fewer startup items = higher score
    try {
        $startupItems = @()

        # Registry Run keys
        $runKeys = @(
            'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run',
            'HKLM:\Software\Microsoft\Windows\CurrentVersion\Run'
        )

        foreach ($key in $runKeys) {
            if (Test-Path $key) {
                $items = Get-ItemProperty -Path $key -ErrorAction SilentlyContinue
                $props = $items.PSObject.Properties | Where-Object { $_.Name -notmatch '^PS' }
                $startupItems += $props
            }
        }

        $startupCount = $startupItems.Count
        $startupScore = [math]::Max(0, 15 - ($startupCount / 2))

        $scores['Startup'] = @{
            Score = [math]::Round($startupScore, 1)
            MaxScore = 15
            Value = "$startupCount items"
            Status = switch ($startupCount) {
                { $_ -lt 10 } { 'Good' }
                { $_ -lt 20 } { 'Fair' }
                default { 'Many' }
            }
        }

        if ($startupCount -gt 20) {
            $recommendations += "Many startup programs detected. Consider disabling unnecessary ones."
        }
    }
    catch {
        $scores['Startup'] = @{ Score = 7; MaxScore = 15; Value = 'Unknown'; Status = 'Unknown' }
    }

    # System Uptime Score (0-15 points)
    # Regular restarts = higher score
    try {
        $bootTime = (Get-CimInstance -ClassName Win32_OperatingSystem -ErrorAction SilentlyContinue).LastBootUpTime
        $uptime = (Get-Date) - $bootTime
        $uptimeDays = $uptime.TotalDays

        $uptimeScore = switch ($uptimeDays) {
            { $_ -lt 3 } { 15 }
            { $_ -lt 7 } { 12 }
            { $_ -lt 14 } { 8 }
            { $_ -lt 30 } { 4 }
            default { 0 }
        }

        $uptimeStr = if ($uptimeDays -ge 1) {
            "$([math]::Floor($uptimeDays)) days, $($uptime.Hours) hours"
        } else {
            "$($uptime.Hours) hours, $($uptime.Minutes) minutes"
        }

        $scores['Uptime'] = @{
            Score = $uptimeScore
            MaxScore = 15
            Value = $uptimeStr
            Status = switch ($uptimeDays) {
                { $_ -lt 7 } { 'Good' }
                { $_ -lt 14 } { 'Fair' }
                default { 'Long' }
            }
        }

        if ($uptimeDays -gt 14) {
            $recommendations += "System hasn't been restarted in over 2 weeks. Consider rebooting."
        }
    }
    catch {
        $scores['Uptime'] = @{ Score = 7; MaxScore = 15; Value = 'Unknown'; Status = 'Unknown' }
    }

    # Calculate total score
    $totalScore = ($scores.Values | ForEach-Object { $_.Score } | Measure-Object -Sum).Sum
    $totalMaxScore = ($scores.Values | ForEach-Object { $_.MaxScore } | Measure-Object -Sum).Sum
    $normalizedScore = [math]::Round(($totalScore / $totalMaxScore) * 100)

    $overallStatus = switch ($normalizedScore) {
        { $_ -ge 80 } { 'Excellent' }
        { $_ -ge 60 } { 'Good' }
        { $_ -ge 40 } { 'Fair' }
        default { 'Poor' }
    }

    $result = @{
        Score = $normalizedScore
        MaxScore = 100
        Status = $overallStatus
        Recommendations = $recommendations
    }

    if ($Detailed) {
        $result.Components = $scores
    }

    return $result
}

function Get-HealthGrade {
    <#
    .SYNOPSIS
        Returns a letter grade based on health score.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [int]$Score
    )

    switch ($Score) {
        { $_ -ge 90 } { return 'A' }
        { $_ -ge 80 } { return 'B' }
        { $_ -ge 70 } { return 'C' }
        { $_ -ge 60 } { return 'D' }
        default { return 'F' }
    }
}

function Get-HealthColor {
    <#
    .SYNOPSIS
        Returns a color based on health score.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [int]$Score
    )

    switch ($Score) {
        { $_ -ge 80 } { return 'Green' }
        { $_ -ge 60 } { return 'Yellow' }
        { $_ -ge 40 } { return 'Magenta' }
        default { return 'Red' }
    }
}
