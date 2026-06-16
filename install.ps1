# install.ps1 — install vanic on Windows
#
# Usage (run in PowerShell as a normal user):
#   irm https://raw.githubusercontent.com/enthusiasticgeek/vani-compiler/main/install.ps1 | iex
#
# Or with a custom install directory:
#   $env:VANIC_INSTALL = "C:\Tools\vanic"; irm .../install.ps1 | iex

$ErrorActionPreference = "Stop"

$Repo   = "enthusiasticgeek/vani-compiler"
$BinName = "vanic.exe"
$Archive = "vanic-windows-x86_64.zip"

# Default install dir: %LOCALAPPDATA%\vanic\bin  (no admin required)
$InstallDir = if ($env:VANIC_INSTALL) { $env:VANIC_INSTALL } `
              else { Join-Path $env:LOCALAPPDATA "vanic\bin" }

# ── fetch latest tag ──────────────────────────────────────────────────────────
Write-Host "Fetching latest release tag…"
$ApiUrl  = "https://api.github.com/repos/$Repo/releases/latest"
$Release = Invoke-RestMethod -Uri $ApiUrl -UseBasicParsing
$Tag     = $Release.tag_name
if (-not $Tag) { throw "Could not determine latest release tag." }

$DownloadUrl = "https://github.com/$Repo/releases/download/$Tag/$Archive"

# ── download ──────────────────────────────────────────────────────────────────
$TmpDir  = Join-Path $env:TEMP "vanic-install-$([System.IO.Path]::GetRandomFileName())"
$ZipPath = Join-Path $TmpDir $Archive
New-Item -ItemType Directory -Force -Path $TmpDir | Out-Null

Write-Host "Downloading vanic $Tag…"
Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath -UseBasicParsing

# ── extract & install ─────────────────────────────────────────────────────────
Expand-Archive -Path $ZipPath -DestinationPath $TmpDir -Force
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item (Join-Path $TmpDir $BinName) (Join-Path $InstallDir $BinName) -Force

Remove-Item $TmpDir -Recurse -Force

# ── PATH persistence ──────────────────────────────────────────────────────────
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable(
        "PATH", "$InstallDir;$UserPath", "User")
    Write-Host "Added $InstallDir to your user PATH."
    Write-Host "Restart your terminal (or run: refreshenv) for PATH to take effect."
} else {
    Write-Host "$InstallDir is already in your user PATH."
}

Write-Host ""
Write-Host "Installed vanic $Tag → $InstallDir\$BinName"
Write-Host "Run:  vanic --version"
