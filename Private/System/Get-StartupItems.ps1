function Get-StartupItems {
    <#
    .SYNOPSIS
        Gets all startup items from various sources.
    .DESCRIPTION
        Enumerates startup items from Registry Run keys, Startup folders,
        Scheduled Tasks with login triggers, and services.
    .PARAMETER Source
        Filter by source type.
    .PARAMETER IncludeDisabled
        Include disabled startup items.
    #>
    [CmdletBinding()]
    param(
        [Parameter()]
        [ValidateSet('All', 'Registry', 'StartupFolder', 'ScheduledTask', 'Service')]
        [string]$Source = 'All',

        [Parameter()]
        [switch]$IncludeDisabled
    )

    $results = @()

    # Registry Run keys
    if ($Source -in @('All', 'Registry')) {
        $registryLocations = @(
            @{ Path = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'; Scope = 'User'; Enabled = $true }
            @{ Path = 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Run'; Scope = 'Machine'; Enabled = $true }
            @{ Path = 'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run'; Scope = 'Machine (32-bit)'; Enabled = $true }
            @{ Path = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\RunOnce'; Scope = 'User'; Enabled = $true; RunOnce = $true }
            @{ Path = 'HKLM:\Software\Microsoft\Windows\CurrentVersion\RunOnce'; Scope = 'Machine'; Enabled = $true; RunOnce = $true }
        )

        # Disabled items location
        $disabledPath = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run'
        $disabledItems = @{}

        if (Test-Path $disabledPath) {
            try {
                $disabledProps = Get-ItemProperty -Path $disabledPath -ErrorAction SilentlyContinue
                foreach ($prop in $disabledProps.PSObject.Properties | Where-Object { $_.Name -notmatch '^PS' }) {
                    # If first bytes are not 02 00, item is disabled
                    if ($prop.Value -is [byte[]] -and $prop.Value.Length -gt 0) {
                        $isDisabled = $prop.Value[0] -ne 2
                        $disabledItems[$prop.Name] = $isDisabled
                    }
                }
            }
            catch { }
        }

        foreach ($location in $registryLocations) {
            if (-not (Test-Path $location.Path)) { continue }

            try {
                $props = Get-ItemProperty -Path $location.Path -ErrorAction SilentlyContinue
                foreach ($prop in $props.PSObject.Properties | Where-Object { $_.Name -notmatch '^PS' }) {
                    $command = $prop.Value
                    $name = $prop.Name

                    # Check if disabled
                    $isEnabled = -not $disabledItems[$name]

                    if (-not $IncludeDisabled -and -not $isEnabled) { continue }

                    # Extract executable path
                    $exePath = $command -replace '^"([^"]+)".*', '$1'
                    if ($exePath -eq $command) {
                        $exePath = ($command -split ' ')[0]
                    }

                    # Check if file exists and get info
                    $fileExists = $false
                    $publisher = 'Unknown'
                    $isSigned = $false

                    if (Test-Path $exePath -ErrorAction SilentlyContinue) {
                        $fileExists = $true
                        try {
                            $sig = Get-AuthenticodeSignature -FilePath $exePath -ErrorAction SilentlyContinue
                            if ($sig.Status -eq 'Valid') {
                                $isSigned = $true
                                $publisher = $sig.SignerCertificate.Subject -replace '^CN=([^,]+).*', '$1'
                            }
                        }
                        catch { }
                    }

                    # Determine category
                    $category = if ($publisher -match 'Microsoft') { 'Microsoft' }
                                elseif ($isSigned) { 'Third-party' }
                                else { 'Unknown' }

                    $results += [PSCustomObject]@{
                        Name = $name
                        Command = $command
                        Path = $exePath
                        Source = 'Registry'
                        SourcePath = $location.Path
                        Scope = $location.Scope
                        Enabled = $isEnabled
                        FileExists = $fileExists
                        Publisher = $publisher
                        IsSigned = $isSigned
                        Category = $category
                        RunOnce = $location.RunOnce -eq $true
                        Impact = 'Unknown'  # Would need Task Scheduler data
                    }
                }
            }
            catch { }
        }
    }

    # Startup folders
    if ($Source -in @('All', 'StartupFolder')) {
        $startupFolders = @(
            @{ Path = [Environment]::GetFolderPath('Startup'); Scope = 'User' }
            @{ Path = [Environment]::GetFolderPath('CommonStartup'); Scope = 'All Users' }
        )

        foreach ($folder in $startupFolders) {
            if (-not (Test-Path $folder.Path)) { continue }

            $items = Get-ChildItem -Path $folder.Path -ErrorAction SilentlyContinue |
                Where-Object { $_.Extension -in @('.lnk', '.exe', '.bat', '.cmd', '.vbs') }

            foreach ($item in $items) {
                $targetPath = $item.FullName
                $name = $item.BaseName

                # Resolve shortcut target
                if ($item.Extension -eq '.lnk') {
                    try {
                        $shell = New-Object -ComObject WScript.Shell
                        $shortcut = $shell.CreateShortcut($item.FullName)
                        $targetPath = $shortcut.TargetPath
                        [System.Runtime.InteropServices.Marshal]::ReleaseComObject($shell) | Out-Null
                    }
                    catch { }
                }

                $fileExists = Test-Path $targetPath -ErrorAction SilentlyContinue
                $publisher = 'Unknown'
                $isSigned = $false

                if ($fileExists -and $targetPath -match '\.exe$') {
                    try {
                        $sig = Get-AuthenticodeSignature -FilePath $targetPath -ErrorAction SilentlyContinue
                        if ($sig.Status -eq 'Valid') {
                            $isSigned = $true
                            $publisher = $sig.SignerCertificate.Subject -replace '^CN=([^,]+).*', '$1'
                        }
                    }
                    catch { }
                }

                $category = if ($publisher -match 'Microsoft') { 'Microsoft' }
                            elseif ($isSigned) { 'Third-party' }
                            else { 'Unknown' }

                $results += [PSCustomObject]@{
                    Name = $name
                    Command = $targetPath
                    Path = $targetPath
                    Source = 'Startup Folder'
                    SourcePath = $item.FullName
                    Scope = $folder.Scope
                    Enabled = $true
                    FileExists = $fileExists
                    Publisher = $publisher
                    IsSigned = $isSigned
                    Category = $category
                    RunOnce = $false
                    Impact = 'Unknown'
                }
            }
        }
    }

    # Scheduled Tasks with logon triggers
    if ($Source -in @('All', 'ScheduledTask')) {
        try {
            $tasks = Get-ScheduledTask -ErrorAction SilentlyContinue |
                Where-Object {
                    $_.Triggers | Where-Object { $_.CimClass.CimClassName -eq 'MSFT_TaskLogonTrigger' }
                }

            foreach ($task in $tasks) {
                $action = $task.Actions | Where-Object { $_.CimClass.CimClassName -eq 'MSFT_TaskExecAction' } | Select-Object -First 1
                if (-not $action) { continue }

                $path = $action.Execute
                $args = $action.Arguments
                $command = if ($args) { "$path $args" } else { $path }

                $isEnabled = $task.State -ne 'Disabled'
                if (-not $IncludeDisabled -and -not $isEnabled) { continue }

                $fileExists = Test-Path $path -ErrorAction SilentlyContinue

                $results += [PSCustomObject]@{
                    Name = $task.TaskName
                    Command = $command
                    Path = $path
                    Source = 'Scheduled Task'
                    SourcePath = $task.TaskPath
                    Scope = if ($task.Principal.UserId -match 'Users|Everyone') { 'All Users' } else { 'User' }
                    Enabled = $isEnabled
                    FileExists = $fileExists
                    Publisher = $task.Author
                    IsSigned = $false
                    Category = if ($task.Author -match 'Microsoft') { 'Microsoft' } else { 'Third-party' }
                    RunOnce = $false
                    Impact = 'Unknown'
                }
            }
        }
        catch { }
    }

    # Auto-start services (brief summary only)
    if ($Source -in @('All', 'Service')) {
        try {
            $services = Get-CimInstance -ClassName Win32_Service -Filter "StartMode='Auto'" -ErrorAction SilentlyContinue |
                Where-Object { $_.State -eq 'Running' } |
                Select-Object -First 20  # Limit for performance

            foreach ($service in $services) {
                $path = $service.PathName -replace '^"([^"]+)".*', '$1'
                $fileExists = Test-Path $path -ErrorAction SilentlyContinue

                $results += [PSCustomObject]@{
                    Name = $service.DisplayName
                    Command = $service.PathName
                    Path = $path
                    Source = 'Service'
                    SourcePath = "Services\$($service.Name)"
                    Scope = 'System'
                    Enabled = $service.State -eq 'Running'
                    FileExists = $fileExists
                    Publisher = 'Unknown'
                    IsSigned = $false
                    Category = if ($service.Name -match '^(wuauserv|bits|Dnscache|WSearch)') { 'Microsoft' } else { 'Unknown' }
                    RunOnce = $false
                    Impact = 'Unknown'
                    ServiceName = $service.Name
                }
            }
        }
        catch { }
    }

    return $results | Sort-Object Source, Name
}

function Get-StartupImpact {
    <#
    .SYNOPSIS
        Estimates the boot impact of startup items.
    .DESCRIPTION
        Uses Task Manager data if available, otherwise estimates based on executable size.
    #>
    [CmdletBinding()]
    param(
        [Parameter(ValueFromPipeline)]
        [object]$StartupItem
    )

    process {
        # This would ideally use actual boot trace data
        # For now, provide a simple estimate based on file size
        $impact = 'Unknown'
        $impactSeconds = 0

        if ($StartupItem.Path -and (Test-Path $StartupItem.Path -ErrorAction SilentlyContinue)) {
            try {
                $size = (Get-Item $StartupItem.Path -ErrorAction SilentlyContinue).Length

                # Very rough estimate: larger files = more impact
                $impactSeconds = switch ($size) {
                    { $_ -gt 100MB } { [math]::Round(($_ / 1MB) * 0.05, 1) }
                    { $_ -gt 50MB } { [math]::Round(($_ / 1MB) * 0.03, 1) }
                    { $_ -gt 10MB } { [math]::Round(($_ / 1MB) * 0.02, 1) }
                    default { [math]::Round(($_ / 1MB) * 0.01, 1) }
                }

                $impact = switch ($impactSeconds) {
                    { $_ -gt 3 } { 'High' }
                    { $_ -gt 1.5 } { 'Medium' }
                    default { 'Low' }
                }
            }
            catch { }
        }

        return @{
            Impact = $impact
            EstimatedSeconds = $impactSeconds
        }
    }
}
