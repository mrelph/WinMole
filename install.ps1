#Requires -Version 5.1
<#
.SYNOPSIS
    Install or update WinMole from GitHub Releases.
.DESCRIPTION
    Downloads the latest WinMole binary from GitHub Releases and installs it
    to the specified directory. Optionally adds the install directory to the
    user PATH so 'winmole' can be run from any terminal.
.PARAMETER InstallDir
    Directory where winmole.exe will be placed.
    Defaults to %LOCALAPPDATA%\WinMole.
.PARAMETER NoPath
    Skip adding the install directory to the user PATH.
.EXAMPLE
    irm https://raw.githubusercontent.com/mrelph/WinMole/main/install.ps1 | iex
.EXAMPLE
    .\install.ps1 -InstallDir "C:\Tools\WinMole"
.EXAMPLE
    .\install.ps1 -NoPath
#>
param(
    [switch]$NoPath,
    [string]$InstallDir = "$env:LOCALAPPDATA\WinMole"
)

$ErrorActionPreference = 'Stop'

# Ensure TLS 1.2 is available (PowerShell 5.1 defaults to TLS 1.0, which GitHub rejects)
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

# ---------------------------------------------------------------------------
# Branding
# ---------------------------------------------------------------------------

function Show-Brand {
    $logo = @"

    __      __.__        _____         .__
    /  \    /  \__| _____/     \   ____ |  |   ____
    \   \/\/   /  |/    \  Y  /  /  _ \|  | _/ __ \
     \        /|  |   |  \   /  (  <_> )  |_\  ___/
      \__/\  / |__|___|  /\_/    \____/|____/\___  >
           \/          \/                        \/
"@
    Write-Host $logo -ForegroundColor Cyan
    Write-Host "         Windows System Optimization Tool" -ForegroundColor White
    Write-Host ""
}

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

function Write-Info    { param([string]$Message) Write-Host "  [i] $Message" -ForegroundColor Cyan }
function Write-Ok      { param([string]$Message) Write-Host "  [+] $Message" -ForegroundColor Green }
function Write-Warn    { param([string]$Message) Write-Host "  [!] $Message" -ForegroundColor Yellow }
function Write-Err     { param([string]$Message) Write-Host "  [x] $Message" -ForegroundColor Red }
function Write-Step    { param([string]$Message) Write-Host "  [-] $Message" -ForegroundColor White }

# ---------------------------------------------------------------------------
# Main logic wrapped in a try/catch for clean error handling
# ---------------------------------------------------------------------------

try {
    Show-Brand

    # ── 1. Query GitHub for the latest release ────────────────────────────
    Write-Info "Checking for latest release..."

    $apiUrl = "https://api.github.com/repos/mrelph/WinMole/releases/latest"

    $headers = @{
        "Accept"     = "application/vnd.github.v3+json"
        "User-Agent" = "WinMole-Installer/1.0"
    }

    try {
        $release = Invoke-RestMethod -Uri $apiUrl -Headers $headers
    }
    catch {
        $status = $_.Exception.Response.StatusCode.value__
        if ($status -eq 403) {
            Write-Warn "Or download manually: https://github.com/mrelph/WinMole/releases"
            throw "GitHub API rate limit exceeded. Try again in a few minutes."
        }
        if ($status -eq 404) {
            Write-Warn "Visit: https://github.com/mrelph/WinMole/releases"
            throw "No releases found for WinMole."
        }
        throw
    }

    $tagName = $release.tag_name
    $version = if ($tagName.StartsWith("v")) { $tagName.Substring(1) } else { $tagName }
    Write-Ok "Latest version: $version"

    # ── 2. Find the Windows x64 binary asset ──────────────────────────────
    $asset = $release.assets | Where-Object { $_.name -match '\.exe$' } | Select-Object -First 1

    if (-not $asset) {
        Write-Warn "Available assets:"
        foreach ($a in $release.assets) {
            Write-Warn "  - $($a.name)"
        }
        Write-Warn "Download manually: $($release.html_url)"
        throw "No .exe asset found in release $tagName."
    }

    $downloadUrl = $asset.browser_download_url
    $assetName   = $asset.name
    $assetSize   = $asset.size

    $sizeMB = [math]::Round($assetSize / 1MB, 2)
    Write-Info "Asset: $assetName ($sizeMB MB)"

    # ── 3. Prepare install directory ──────────────────────────────────────
    $installPath = Join-Path $InstallDir "winmole.exe"

    if (-not (Test-Path $InstallDir)) {
        Write-Step "Creating directory: $InstallDir"
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }

    # Check if an existing version is installed
    if (Test-Path $installPath) {
        Write-Warn "Existing installation found. It will be replaced."
    }

    # ── 4. Download the binary ────────────────────────────────────────────
    Write-Info "Downloading winmole $version..."

    $tempFile = Join-Path $env:TEMP "winmole-download-$([System.Guid]::NewGuid().ToString('N').Substring(0,8)).exe"

    try {
        # Use .NET WebClient for progress on PS 5.1; Invoke-WebRequest for PS 7+
        if ($PSVersionTable.PSVersion.Major -ge 7) {
            Invoke-WebRequest -Uri $downloadUrl -OutFile $tempFile -UseBasicParsing
        }
        else {
            # PowerShell 5.1 - Invoke-WebRequest shows progress bar by default
            $ProgressPreference = 'Continue'
            Invoke-WebRequest -Uri $downloadUrl -OutFile $tempFile -UseBasicParsing
        }
    }
    catch {
        if (Test-Path $tempFile) { Remove-Item $tempFile -Force -ErrorAction SilentlyContinue }
        throw "Download failed: $_"
    }

    # Verify the download produced a file
    if (-not (Test-Path $tempFile)) {
        throw "Downloaded file not found. Something went wrong."
    }

    $downloadedSize = (Get-Item $tempFile).Length
    if ($downloadedSize -eq 0) {
        Remove-Item $tempFile -Force -ErrorAction SilentlyContinue
        throw "Downloaded file is empty."
    }

    Write-Ok "Download complete ($([math]::Round($downloadedSize / 1MB, 2)) MB)"

    # ── 4.5. Verify SHA-256 checksum if the release publishes one ─────────
    $checksumAsset = $release.assets | Where-Object { $_.name -eq "$assetName.sha256" } | Select-Object -First 1

    if ($checksumAsset) {
        Write-Info "Verifying SHA-256 checksum..."
        $checksumText = Invoke-RestMethod -Uri $checksumAsset.browser_download_url -Headers $headers
        $expected = ([string]$checksumText -split '\s+')[0].ToLower()
        $actual = (Get-FileHash $tempFile -Algorithm SHA256).Hash.ToLower()

        if ($actual -ne $expected) {
            Remove-Item $tempFile -Force -ErrorAction SilentlyContinue
            throw "Checksum mismatch! Expected $expected but got $actual. Aborting install."
        }
        Write-Ok "Checksum verified"
    }
    else {
        Write-Warn "No checksum published for this release - skipping verification"
    }

    # ── 5. Install the binary ─────────────────────────────────────────────
    Write-Step "Installing to: $installPath"

    try {
        # If the target exists, try to remove it first
        if (Test-Path $installPath) {
            Remove-Item $installPath -Force
        }
        Move-Item -Path $tempFile -Destination $installPath -Force
    }
    catch {
        Write-Warn "The file may be in use. Close any running WinMole instances and try again."
        if (Test-Path $tempFile) { Remove-Item $tempFile -Force -ErrorAction SilentlyContinue }
        throw "Failed to install binary: $_"
    }

    Write-Ok "Installed winmole.exe successfully"

    # ── 6. Add to PATH (unless -NoPath) ───────────────────────────────────
    $pathModified = $false

    if ($NoPath) {
        Write-Warn "Skipping PATH modification (-NoPath specified)"
    }
    else {
        $userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
        if (-not $userPath) { $userPath = "" }
        $pathEntries = $userPath -split ";" | Where-Object { $_ -ne "" }

        # Check if the install dir is already in PATH (case-insensitive)
        $alreadyInPath = $pathEntries | Where-Object {
            $_.TrimEnd('\') -ieq $InstallDir.TrimEnd('\')
        }

        if ($alreadyInPath) {
            Write-Ok "Install directory already in PATH"
        }
        else {
            Write-Step "Adding to user PATH: $InstallDir"
            $newPath = ($userPath.TrimEnd(";") + ";$InstallDir").TrimStart(";")
            [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
            # Also update the current session so the user can run winmole immediately
            $env:PATH = "$env:PATH;$InstallDir"
            Write-Ok "Added to user PATH"
            $pathModified = $true
        }
    }

    # ── 7. Done ───────────────────────────────────────────────────────────
    Write-Host ""
    Write-Host "  ============================================" -ForegroundColor Cyan
    Write-Host "    WinMole $version installed successfully!" -ForegroundColor Green
    Write-Host "  ============================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "  Run 'winmole' to get started." -ForegroundColor White
    Write-Host "  Run 'winmole --help' for available commands." -ForegroundColor White
    Write-Host ""

    if ($pathModified) {
        Write-Warn "You may need to restart your terminal for PATH changes to take effect."
        Write-Host ""
    }
}
catch {
    Write-Host ""
    Write-Err "Installation failed: $_"
    Write-Host ""
    Write-Host "  If this problem persists, please:" -ForegroundColor White
    Write-Host "    1. Check your internet connection" -ForegroundColor White
    Write-Host "    2. Try again in a few minutes" -ForegroundColor White
    Write-Host "    3. Download manually from:" -ForegroundColor White
    Write-Host "       https://github.com/mrelph/WinMole/releases" -ForegroundColor Cyan
    Write-Host ""
    # Exit with a failure code when run as a script file (CI), but not when
    # piped through iex - exiting there would close the user's shell.
    if ($MyInvocation.MyCommand.Path) { exit 1 }
}
