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

# ── detect architecture ────────────────────────────────────────────────────────
# RuntimeInformation.OSArchitecture reports the OS's native architecture
# regardless of whether this script is itself running under a 32-bit or
# WOW64-emulated PowerShell host — $env:PROCESSOR_ARCHITECTURE alone can
# lie in that case, so this is the reliable one.
$OsArch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
switch ($OsArch) {
    "X64" {
        $Archive = "vanic-windows-x86_64.zip"
    }
    "Arm64" {
        # No native win-arm64 build yet (see .github/workflows/release.yml) --
        # fall back to the x86_64 build, which runs under Windows 11's
        # (and Windows 10 22H2+'s optional) built-in x86_64-on-ARM64
        # emulation. Not as fast as a native build, but it works.
        Write-Warning "No native Windows-on-ARM64 build of vanic yet -- installing the x86_64 build, which will run under Windows' built-in emulation."
        $Archive = "vanic-windows-x86_64.zip"
    }
    default {
        throw "Unsupported Windows architecture: $OsArch (vanic ships x86_64 and ARM64-via-emulation builds only)."
    }
}

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
