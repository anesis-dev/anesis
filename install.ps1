$ErrorActionPreference = "Stop"

$APP_NAME = "anesis"
$REPO = "anesis-dev/anesis"
$ARCH = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "i686" }

Write-Host "Fetching latest version..." -ForegroundColor Cyan

try {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest"
    $LATEST_VERSION = $release.tag_name
} catch {
    Write-Host "Failed to fetch latest version: $_" -ForegroundColor Red
    exit 1
}

if ([string]::IsNullOrEmpty($LATEST_VERSION)) {
    Write-Host "Failed to fetch latest version" -ForegroundColor Red
    exit 1
}

Write-Host "Installing $APP_NAME $LATEST_VERSION..." -ForegroundColor Green

$DOWNLOAD_URL = "https://github.com/$REPO/releases/download/$LATEST_VERSION/${APP_NAME}-windows-${ARCH}.zip"
$TMP_DIR = Join-Path $env:TEMP ([System.IO.Path]::GetRandomFileName())
New-Item -ItemType Directory -Path $TMP_DIR | Out-Null

Write-Host "Downloading from $DOWNLOAD_URL..." -ForegroundColor Cyan

try {
    $zipPath = Join-Path $TMP_DIR "$APP_NAME.zip"
    Invoke-WebRequest -Uri $DOWNLOAD_URL -OutFile $zipPath
    Expand-Archive -Path $zipPath -DestinationPath $TMP_DIR -Force

    $INSTALL_DIR = Join-Path $env:USERPROFILE ".local\bin"
    if (-not (Test-Path $INSTALL_DIR)) {
        New-Item -ItemType Directory -Path $INSTALL_DIR | Out-Null
    }

    $exeName = "$APP_NAME.exe"
    $sourcePath = Join-Path $TMP_DIR $exeName
    $destPath = Join-Path $INSTALL_DIR $exeName

    if (Test-Path $destPath) {
        Remove-Item $destPath -Force
    }

    Move-Item -Path $sourcePath -Destination $destPath -Force

    Write-Host ""
    Write-Host "✓ $APP_NAME installed successfully to $destPath" -ForegroundColor Green
    Write-Host ""

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$INSTALL_DIR*") {
        Write-Host "Adding $INSTALL_DIR to PATH..." -ForegroundColor Yellow
        [Environment]::SetEnvironmentVariable("Path", "$userPath;$INSTALL_DIR", "User")
        Write-Host "✓ PATH updated. Please restart your terminal." -ForegroundColor Green
    } else {
        Write-Host "$INSTALL_DIR is already in your PATH" -ForegroundColor Green
    }

    Write-Host "Configuring PowerShell completions..." -ForegroundColor Cyan
    try {
        & $destPath completions powershell
        if ($LASTEXITCODE -ne 0) {
            throw "anesis exited with code $LASTEXITCODE"
        }
    } catch {
        Write-Host "PowerShell completions were not configured automatically: $_" -ForegroundColor Yellow
        Write-Host "Run manually: & '$destPath' completions powershell" -ForegroundColor Yellow
    }

} catch {
    Write-Host "Installation failed: $_" -ForegroundColor Red
    exit 1
} finally {
    if (Test-Path $TMP_DIR) {
        Remove-Item -Path $TMP_DIR -Recurse -Force
    }
}

Write-Host ""
Write-Host "You can now run: $APP_NAME" -ForegroundColor Cyan
