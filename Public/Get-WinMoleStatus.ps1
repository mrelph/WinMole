function Get-WinMoleStatus {
    <#
    .SYNOPSIS
        Real-time system health dashboard.
    .DESCRIPTION
        Displays CPU, RAM, GPU, disk I/O, and network statistics with
        a health score and top process information.
    .PARAMETER Live
        Enable live refresh mode.
    .PARAMETER RefreshInterval
        Refresh interval in seconds for live mode.
    .PARAMETER TopProcesses
        Number of top processes to show.
    .PARAMETER Detailed
        Show detailed breakdown of each metric.
    .EXAMPLE
        Get-WinMoleStatus
        Show current system status.
    .EXAMPLE
        Get-WinMoleStatus -Live -RefreshInterval 2
        Show live updating status every 2 seconds.
    #>
    [CmdletBinding()]
    param(
        [Parameter()]
        [switch]$Live,

        [Parameter()]
        [ValidateRange(1, 60)]
        [int]$RefreshInterval = 2,

        [Parameter()]
        [ValidateRange(3, 20)]
        [int]$TopProcesses = 5,

        [Parameter()]
        [switch]$Detailed
    )

    function Get-CurrentStatus {
        $status = @{}

        # CPU
        try {
            $cpu = Get-CimInstance -ClassName Win32_Processor -ErrorAction SilentlyContinue
            $cpuUsage = ($cpu | Measure-Object -Property LoadPercentage -Average).Average
            $cpuName = $cpu[0].Name -replace '\s+', ' '

            $status.CPU = @{
                Usage = [math]::Round($cpuUsage)
                Name = $cpuName
                Cores = ($cpu | Measure-Object -Property NumberOfCores -Sum).Sum
                Threads = ($cpu | Measure-Object -Property NumberOfLogicalProcessors -Sum).Sum
            }
        }
        catch {
            $status.CPU = @{ Usage = 0; Name = 'Unknown'; Cores = 0; Threads = 0 }
        }

        # Memory
        try {
            $os = Get-CimInstance -ClassName Win32_OperatingSystem -ErrorAction SilentlyContinue
            $totalMemGB = [math]::Round($os.TotalVisibleMemorySize / 1MB, 1)
            $freeMemGB = [math]::Round($os.FreePhysicalMemory / 1MB, 1)
            $usedMemGB = $totalMemGB - $freeMemGB
            $usedPercent = [math]::Round(($usedMemGB / $totalMemGB) * 100)

            $status.Memory = @{
                Total = $totalMemGB
                Used = $usedMemGB
                Free = $freeMemGB
                UsedPercent = $usedPercent
            }
        }
        catch {
            $status.Memory = @{ Total = 0; Used = 0; Free = 0; UsedPercent = 0 }
        }

        # Disk
        try {
            $disk = Get-CimInstance -ClassName Win32_PerfFormattedData_PerfDisk_PhysicalDisk -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -eq '_Total' }

            $status.Disk = @{
                ReadSpeed = [math]::Round($disk.DiskReadBytesPersec / 1MB, 1)
                WriteSpeed = [math]::Round($disk.DiskWriteBytesPersec / 1MB, 1)
                Usage = [math]::Round($disk.PercentDiskTime)
                QueueLength = $disk.CurrentDiskQueueLength
            }
        }
        catch {
            $status.Disk = @{ ReadSpeed = 0; WriteSpeed = 0; Usage = 0; QueueLength = 0 }
        }

        # Network
        try {
            $netAdapters = Get-CimInstance -ClassName Win32_PerfFormattedData_Tcpip_NetworkInterface -ErrorAction SilentlyContinue |
                Where-Object { $_.BytesReceivedPersec -gt 0 -or $_.BytesSentPersec -gt 0 }

            $totalReceived = ($netAdapters | Measure-Object -Property BytesReceivedPersec -Sum).Sum
            $totalSent = ($netAdapters | Measure-Object -Property BytesSentPersec -Sum).Sum

            $status.Network = @{
                Download = [math]::Round($totalReceived / 1MB, 2)
                Upload = [math]::Round($totalSent / 1MB, 2)
            }
        }
        catch {
            $status.Network = @{ Download = 0; Upload = 0 }
        }

        # GPU (basic)
        try {
            $gpu = Get-CimInstance -ClassName Win32_VideoController -ErrorAction SilentlyContinue | Select-Object -First 1
            $status.GPU = @{
                Name = $gpu.Name
                DriverVersion = $gpu.DriverVersion
                RAM = [math]::Round($gpu.AdapterRAM / 1GB, 1)
            }
        }
        catch {
            $status.GPU = @{ Name = 'Unknown'; DriverVersion = ''; RAM = 0 }
        }

        # Health Score
        $healthResult = Get-HealthScore -Detailed:$Detailed
        $status.Health = $healthResult

        # Top Processes
        try {
            $status.TopProcesses = Get-Process -ErrorAction SilentlyContinue |
                Sort-Object WorkingSet64 -Descending |
                Select-Object -First $TopProcesses |
                ForEach-Object {
                    @{
                        Name = $_.ProcessName
                        CPU = [math]::Round($_.CPU, 1)
                        Memory = [math]::Round($_.WorkingSet64 / 1MB)
                        Id = $_.Id
                    }
                }
        }
        catch {
            $status.TopProcesses = @()
        }

        # Uptime
        try {
            $bootTime = (Get-CimInstance -ClassName Win32_OperatingSystem -ErrorAction SilentlyContinue).LastBootUpTime
            $uptime = (Get-Date) - $bootTime

            $status.Uptime = @{
                Days = $uptime.Days
                Hours = $uptime.Hours
                Minutes = $uptime.Minutes
                TotalHours = [math]::Round($uptime.TotalHours, 1)
            }
        }
        catch {
            $status.Uptime = @{ Days = 0; Hours = 0; Minutes = 0; TotalHours = 0 }
        }

        return $status
    }

    function Show-StatusDisplay {
        param($Status)

        Clear-Host

        # Header with health score
        $healthColor = Get-HealthColor -Score $Status.Health.Score
        $healthGrade = Get-HealthGrade -Score $Status.Health.Score

        $width = 54
        $topBorder = [char]0x2554 + ([char]0x2550 * ($width - 2)) + [char]0x2557
        $midBorder = [char]0x2560 + ([char]0x2550 * ($width - 2)) + [char]0x2563
        $bottomBorder = [char]0x255A + ([char]0x2550 * ($width - 2)) + [char]0x255D
        $side = [char]0x2551

        Write-ColorOutput $topBorder -ForegroundColor Cyan

        # Title line
        $title = " WinMole Status "
        $titlePad = ($width - 2 - $title.Length) / 2
        $titleLine = "$side" + (' ' * [math]::Floor($titlePad)) + $title + (' ' * [math]::Ceiling($titlePad)) + "$side"
        Write-ColorOutput $titleLine -ForegroundColor Cyan

        Write-ColorOutput $midBorder -ForegroundColor Cyan

        # Health Score
        $healthBar = Write-InlineProgressBar -Percent $Status.Health.Score -Width 20 -Color $healthColor
        $healthLabel = "  Health Score: $($Status.Health.Score)/100"
        Write-ColorOutput "$side$healthLabel $healthBar $($Status.Health.Status.PadRight(10))$side" -ForegroundColor Cyan

        Write-ColorOutput $midBorder -ForegroundColor Cyan

        # CPU
        $cpuBar = Write-InlineProgressBar -Percent $Status.CPU.Usage -Width 20
        $cpuInfo = "$($Status.CPU.Cores)C/$($Status.CPU.Threads)T"
        $cpuLine = "  CPU   $($Status.CPU.Usage.ToString().PadLeft(3))% $cpuBar  $cpuInfo"
        Write-Host "$side$($cpuLine.PadRight($width - 2))$side"

        # Memory
        $memBar = Write-InlineProgressBar -Percent $Status.Memory.UsedPercent -Width 20
        $memInfo = "$($Status.Memory.Used)/$($Status.Memory.Total) GB"
        $memLine = "  RAM   $($Status.Memory.UsedPercent.ToString().PadLeft(3))% $memBar  $memInfo"
        Write-Host "$side$($memLine.PadRight($width - 2))$side"

        # Disk
        $diskBar = Write-InlineProgressBar -Percent $Status.Disk.Usage -Width 20
        $diskInfo = "R:$($Status.Disk.ReadSpeed) W:$($Status.Disk.WriteSpeed) MB/s"
        $diskLine = "  Disk  $($Status.Disk.Usage.ToString().PadLeft(3))% $diskBar  $diskInfo"
        Write-Host "$side$($diskLine.PadRight($width - 2))$side"

        # Network
        $netLine = "  Net   $([char]0x2191)$($Status.Network.Upload) MB/s  $([char]0x2193)$($Status.Network.Download) MB/s"
        Write-Host "$side$($netLine.PadRight($width - 2))$side"

        Write-ColorOutput $midBorder -ForegroundColor Cyan

        # Top Processes
        $procHeader = "  Top Processes by Memory"
        Write-Host "$side$($procHeader.PadRight($width - 2))$side"

        foreach ($proc in $Status.TopProcesses) {
            $procName = $proc.Name
            if ($procName.Length -gt 20) { $procName = $procName.Substring(0, 17) + '...' }
            $procLine = "    $($procName.PadRight(20)) $($proc.Memory.ToString().PadLeft(6)) MB"
            Write-Host "$side$($procLine.PadRight($width - 2))$side"
        }

        Write-ColorOutput $midBorder -ForegroundColor Cyan

        # Uptime
        $uptimeStr = "$($Status.Uptime.Days)d $($Status.Uptime.Hours)h $($Status.Uptime.Minutes)m"
        $uptimeLine = "  Uptime: $uptimeStr"
        Write-Host "$side$($uptimeLine.PadRight($width - 2))$side"

        Write-ColorOutput $bottomBorder -ForegroundColor Cyan

        if ($Live) {
            Write-Host ""
            Write-ColorOutput "  Press Ctrl+C to exit" -ForegroundColor BrightBlack
        }
    }

    # Main execution
    if ($Live) {
        try {
            while ($true) {
                $status = Get-CurrentStatus
                Show-StatusDisplay -Status $status
                Start-Sleep -Seconds $RefreshInterval
            }
        }
        catch {
            # Ctrl+C pressed
            Write-Host ""
            Write-WinMoleStatus -Message "Status monitor stopped" -Type Info
        }
    }
    else {
        $status = Get-CurrentStatus
        Show-StatusDisplay -Status $status

        # Recommendations
        if ($status.Health.Recommendations -and $status.Health.Recommendations.Count -gt 0) {
            Write-Host ""
            Write-WinMoleSection -Title "Recommendations"
            foreach ($rec in $status.Health.Recommendations) {
                Write-WinMoleStatus -Message $rec -Type Warning
            }
        }

        # Return status object for scripting
        return [PSCustomObject]@{
            HealthScore = $status.Health.Score
            HealthStatus = $status.Health.Status
            CPUUsage = $status.CPU.Usage
            MemoryUsedPercent = $status.Memory.UsedPercent
            MemoryUsedGB = $status.Memory.Used
            MemoryTotalGB = $status.Memory.Total
            DiskUsage = $status.Disk.Usage
            DiskReadMBps = $status.Disk.ReadSpeed
            DiskWriteMBps = $status.Disk.WriteSpeed
            NetworkDownloadMBps = $status.Network.Download
            NetworkUploadMBps = $status.Network.Upload
            UptimeHours = $status.Uptime.TotalHours
            TopProcesses = $status.TopProcesses
            Recommendations = $status.Health.Recommendations
        }
    }
}
