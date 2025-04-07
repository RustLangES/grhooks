<#
.SYNOPSIS
GRHooks Windows Installation Script

.DESCRIPTION
Installs GRHooks webhook server on Windows systems with optional service configuration
#>

# Set Error Action
$ErrorActionPreference = "Stop"

# Colors
$Host.UI.RawUI.ForegroundColor = "White"

# Configuration - Can be overridden by environment variables
$REPO = "RustLangES/grhooks"
$VERSION = if ($env:VERSION) { $env:VERSION } else { "latest" }
$INSTALL_DIR = if ($env:INSTALL_DIR) { $env:INSTALL_DIR } else { "$env:ProgramFiles\GRHooks" }
$CONFIG_DIR = if ($env:CONFIG_DIR) { $env:CONFIG_DIR } else { "$env:ProgramData\GRHooks\config" }
$SERVICE_NAME = if ($env:SERVICE_NAME) { $env:SERVICE_NAME } else { "GRHooks" }
$LOG_LEVEL = if ($env:LOG_LEVEL) { $env:LOG_LEVEL } else { "info" }

# Display configuration
Write-Host "Install Configuration:" -ForegroundColor Yellow
Write-Host "Version:       " -NoNewline; Write-Host $VERSION -ForegroundColor Green
Write-Host "Install Dir:   " -NoNewline; Write-Host $INSTALL_DIR -ForegroundColor Green
Write-Host "Configuration: " -NoNewline; Write-Host $CONFIG_DIR -ForegroundColor Green
Write-Host "Service:       " -NoNewline; Write-Host $SERVICE_NAME -ForegroundColor Green
Write-Host "Log Level:     " -NoNewline; Write-Host $LOG_LEVEL -ForegroundColor Green
Write-Host ""

# Check if running on Windows
if ($PSVersionTable.PSVersion.Major -lt 5 -or -not $IsWindows) {
    Write-Host "Error: This script requires Windows PowerShell 5.1 or later" -ForegroundColor Red
    exit 1
}

# Determine architecture
$ARCH = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "i686" }

# Determine package type (Windows always uses zip)
$PKG_TYPE = "zip"

function Prompt-YesNo {
    param(
        [string]$Question
    )

    do {
        $response = Read-Host "$Question (Y/N)"
    } while ($response -notmatch '^[yYnN]$')

    return $response -match '^[yY]$'
}

if (-not (Prompt-YesNo "Do you want to continue with the installation?")) {
    Write-Host "Installation aborted." -ForegroundColor Red
    exit 1
}

# Step 1: Download
Write-Host "[1/4] Downloading..." -ForegroundColor Yellow

if ($VERSION -eq "latest") {
    $releaseInfo = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest"
    $DOWNLOAD_URL = $releaseInfo.assets | Where-Object {
        $_.name -match "windows" -and $_.name -match $ARCH -and $_.name.EndsWith(".zip")
    } | Select-Object -First 1 -ExpandProperty browser_download_url
} else {
    $DOWNLOAD_URL = "https://github.com/$REPO/releases/download/$VERSION/grhooks_${VERSION}_${ARCH}_windows.zip"
}

if (-not $DOWNLOAD_URL) {
    Write-Host "Cannot find package for your system (Arch: $ARCH)" -ForegroundColor Red
    exit 1
}

Write-Host "Downloading: $DOWNLOAD_URL"
$TEMP_DIR = Join-Path $env:TEMP "grhooks-install"
New-Item -ItemType Directory -Path $TEMP_DIR -Force | Out-Null
$zipFile = Join-Path $TEMP_DIR "grhooks.zip"

try {
    Invoke-WebRequest -Uri $DOWNLOAD_URL -OutFile $zipFile
} catch {
    Write-Host "Failed to download package: $_" -ForegroundColor Red
    exit 1
}

# Step 2: Install
Write-Host "[2/4] Installing..." -ForegroundColor Yellow

try {
    # Create installation directory
    New-Item -ItemType Directory -Path $INSTALL_DIR -Force | Out-Null

    # Extract zip file
    Expand-Archive -Path $zipFile -DestinationPath $TEMP_DIR -Force

    # Copy files to installation directory
    Copy-Item -Path "$TEMP_DIR\grhooks.exe" -Destination $INSTALL_DIR -Force

    # Add to PATH if not already present
    $path = [Environment]::GetEnvironmentVariable("PATH", "Machine")
    if ($path -notlike "*$INSTALL_DIR*") {
        [Environment]::SetEnvironmentVariable("PATH", "$path;$INSTALL_DIR", "Machine")
        $env:PATH += ";$INSTALL_DIR"
    }
} catch {
    Write-Host "Installation failed: $_" -ForegroundColor Red
    exit 1
}

# Step 3: Configuration
Write-Host "[3/4] Creating Config Directory..." -ForegroundColor Yellow

try {
    New-Item -ItemType Directory -Path $CONFIG_DIR -Force | Out-Null

# Step 4: Service Configuration
if (Prompt-YesNo "Do you want to configure GRHooks as a Windows service?") {
    Write-Host "[4/4] Configuring Windows service..." -ForegroundColor Yellow

    # Check if service already exists
    $service = Get-Service -Name $SERVICE_NAME -ErrorAction SilentlyContinue

    if ($service) {
        Write-Host "Service $SERVICE_NAME already exists" -ForegroundColor Yellow
        if (Prompt-YesNo "Do you want to reconfigure it?") {
            Stop-Service -Name $SERVICE_NAME -Force -ErrorAction SilentlyContinue
            & sc.exe delete $SERVICE_NAME | Out-Null
        } else {
            exit 0
        }
    }

    # Create service
    try {
        $serviceArgs = "`"$CONFIG_DIR`""

        New-Service -Name $SERVICE_NAME `
                    -BinaryPathName "`"$INSTALL_DIR\grhooks.exe`" $serviceArgs" `
                    -DisplayName "GRHooks Webhook Server" `
                    -Description "GRHooks Webhook Server Service" `
                    -StartupType Automatic `
                    -ErrorAction Stop | Out-Null

        # Configure service recovery
        & sc.exe failure $SERVICE_NAME reset= 60 actions= restart/5000 | Out-Null

        # Set environment variable
        [Environment]::SetEnvironmentVariable("LOG", $LOG_LEVEL, "Machine")

        Write-Host "Service created successfully" -ForegroundColor Green

        if (Prompt-YesNo "Do you want to start the service now?")) {
            Start-Service -Name $SERVICE_NAME
            Write-Host "Service started. You can view the logs in Event Viewer." -ForegroundColor Green
        }
    } catch {
        Write-Host "Service configuration failed: $_" -ForegroundColor Red
    }
} else {
    Write-Host "[4/4] Skipping service configuration." -ForegroundColor Yellow
}

# Cleanup
Remove-Item -Path $TEMP_DIR -Recurse -Force -ErrorAction SilentlyContinue

Write-Host "Installation completed!" -ForegroundColor Green
Write-Host ""
Write-Host "Post your manifests here: " -NoNewline; Write-Host $CONFIG_DIR -ForegroundColor Yellow
Write-Host "Binary:       " -NoNewline; Write-Host "$INSTALL_DIR\grhooks.exe" -ForegroundColor Yellow
if (Get-Service -Name $SERVICE_NAME -ErrorAction SilentlyContinue) {
    Write-Host "Service:      " -NoNewline; Write-Host "Get-Service $SERVICE_NAME" -ForegroundColor Yellow
}
