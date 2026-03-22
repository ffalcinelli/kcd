$ErrorActionPreference = "Stop"
$GitHubRepo = "ffalcinelli/kcd"

Write-Host "Installing kcd..." -ForegroundColor Cyan

# kcd releases x86_64 for windows
$Target = "x86_64-pc-windows-msvc"

$InstallDir = Join-Path $HOME ".local\bin"
if (-Not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

$DownloadUrl = "https://github.com/$GitHubRepo/releases/latest/download/kcd-${Target}.zip"
$TempDir = Join-Path $env:TEMP "kcd-install-$(New-Guid)"
New-Item -ItemType Directory -Force -Path $TempDir | Out-Null
$ZipFile = Join-Path $TempDir "kcd.zip"

Write-Host "Downloading $DownloadUrl..."
Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipFile

Write-Host "Extracting..."
Expand-Archive -Path $ZipFile -DestinationPath $TempDir -Force

$KcdBin = Get-ChildItem -Path $TempDir -Recurse -Filter "kcd.exe" | Select-Object -First 1
if (-Not $KcdBin) {
    Write-Host "Error: kcd.exe not found in archive." -ForegroundColor Red
    Remove-Item -Path $TempDir -Recurse -Force
    exit 1
}

Move-Item -Path $KcdBin.FullName -Destination (Join-Path $InstallDir "kcd.exe") -Force
Remove-Item -Path $TempDir -Recurse -Force

Write-Host "Successfully installed kcd to $InstallDir\kcd.exe" -ForegroundColor Green

# Check Path
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notmatch [regex]::Escape($InstallDir)) {
    Write-Host "`nWARNING: The installation directory ($InstallDir) is not in your PATH." -ForegroundColor Yellow
    Write-Host "To add it to your current session, run:"
    Write-Host "  `$env:PATH += `";$InstallDir`""
    Write-Host "To add it permanently, add it to your User PATH via Windows Settings."
}
