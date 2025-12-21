<# 
.SYNOPSIS
    Calvin CLI Windows Installer
.DESCRIPTION
    Downloads and installs Calvin CLI to a location already in PATH (if possible),
    avoiding the need to restart terminals.
.EXAMPLE
    irm https://raw.githubusercontent.com/64andrewwalker/calvin/main/scripts/install-windows.ps1 | iex
#>

$ErrorActionPreference = 'Stop'
$Version = "nightly"
$RepoUrl = "https://github.com/64andrewwalker/calvin"
$BinaryName = "calvin.exe"
$ZipName = "calvin-x86_64-pc-windows-msvc.zip"

Write-Host ""
Write-Host "  ╭─────────────────────────────────────╮" -ForegroundColor Cyan
Write-Host "  │       Calvin CLI Installer          │" -ForegroundColor Cyan
Write-Host "  ╰─────────────────────────────────────╯" -ForegroundColor Cyan
Write-Host ""

# Strategy 1: Use .cargo/bin if it exists and is in PATH
$CargoDir = "$env:USERPROFILE\.cargo\bin"
$ScoopShims = "$env:USERPROFILE\scoop\shims"
$LocalAppData = "$env:LOCALAPPDATA\calvin"

function Test-InPath($dir) {
    $env:Path -split ';' | Where-Object { $_ -eq $dir }
}

# Determine best install location
$InstallDir = $null
$NeedPathUpdate = $false

if ((Test-Path $CargoDir) -and (Test-InPath $CargoDir)) {
    Write-Host "  ✓ Found .cargo/bin in PATH, using it" -ForegroundColor Green
    $InstallDir = $CargoDir
} elseif ((Test-Path $ScoopShims) -and (Test-InPath $ScoopShims)) {
    Write-Host "  ✓ Found Scoop shims in PATH, using it" -ForegroundColor Green
    $InstallDir = $ScoopShims
} else {
    Write-Host "  → Using $LocalAppData" -ForegroundColor Yellow
    $InstallDir = $LocalAppData
    $NeedPathUpdate = $true
}

# Create directory if needed
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

# Download
Write-Host ""
Write-Host "  Downloading Calvin..." -ForegroundColor Cyan
$TempZip = "$env:TEMP\calvin-install.zip"
$DownloadUrl = "$RepoUrl/releases/download/$Version/$ZipName"

try {
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempZip -UseBasicParsing
} catch {
    Write-Host "  ✗ Download failed: $_" -ForegroundColor Red
    Write-Host "  → Make sure the nightly release exists at:" -ForegroundColor Yellow
    Write-Host "    $DownloadUrl" -ForegroundColor Yellow
    exit 1
}

# Extract
Write-Host "  Extracting..." -ForegroundColor Cyan
$TempExtract = "$env:TEMP\calvin-extract"
if (Test-Path $TempExtract) { Remove-Item -Recurse -Force $TempExtract }
Expand-Archive -Path $TempZip -DestinationPath $TempExtract -Force

# Find and copy binary
$Binary = Get-ChildItem -Path $TempExtract -Recurse -Filter $BinaryName | Select-Object -First 1
if (-not $Binary) {
    Write-Host "  ✗ Could not find $BinaryName in archive" -ForegroundColor Red
    exit 1
}

Copy-Item -Path $Binary.FullName -Destination "$InstallDir\$BinaryName" -Force

# Update PATH if needed
if ($NeedPathUpdate) {
    $CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($CurrentPath -notlike "*$InstallDir*") {
        Write-Host "  Adding to PATH..." -ForegroundColor Cyan
        [Environment]::SetEnvironmentVariable("Path", "$CurrentPath;$InstallDir", "User")
        
        # Update current session
        $env:Path = "$env:Path;$InstallDir"
        
        # Broadcast to other apps
        Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class WinAPI {
    [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Auto)]
    public static extern IntPtr SendMessageTimeout(
        IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam,
        uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);
    public static void NotifyEnvChange() {
        UIntPtr result;
        SendMessageTimeout((IntPtr)0xffff, 0x1A, UIntPtr.Zero, "Environment", 0x0002, 5000, out result);
    }
}
"@
        [WinAPI]::NotifyEnvChange()
    }
}

# Cleanup
Remove-Item -Force $TempZip -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force $TempExtract -ErrorAction SilentlyContinue

# Verify
Write-Host ""
$InstalledPath = "$InstallDir\$BinaryName"
if (Test-Path $InstalledPath) {
    Write-Host "  ╭─────────────────────────────────────╮" -ForegroundColor Green
    Write-Host "  │  ✓ Calvin installed successfully!   │" -ForegroundColor Green
    Write-Host "  ╰─────────────────────────────────────╯" -ForegroundColor Green
    Write-Host ""
    Write-Host "  Location: $InstalledPath" -ForegroundColor Gray
    Write-Host ""
    
    # Test if it works
    try {
        $ver = & $InstalledPath --version 2>&1
        Write-Host "  Version:  $ver" -ForegroundColor Gray
    } catch {}
    
    Write-Host ""
    if ($NeedPathUpdate) {
        Write-Host "  ⚠ PATH was updated. New terminals will work automatically." -ForegroundColor Yellow
        Write-Host "    For THIS terminal, run:" -ForegroundColor Yellow
        Write-Host "    `$env:Path += `";$InstallDir`"" -ForegroundColor Cyan
    } else {
        Write-Host "  ✓ Ready to use! Try: calvin --help" -ForegroundColor Green
    }
} else {
    Write-Host "  ✗ Installation failed" -ForegroundColor Red
    exit 1
}

Write-Host ""
