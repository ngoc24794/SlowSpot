# scripts/shots.ps1
# Builds release binary, runs deterministic screenshot captures at 1000x680 and 760x520,
# and prints the list of generated screenshot files.

[CmdletBinding()]
param(
    [string]$OutDir = "screenshots"
)

$ErrorActionPreference = "Stop"

$scriptDir = $PSScriptRoot
$projectRoot = Split-Path -Parent $scriptDir

# 1. Build release binary
Write-Host "Building release binary..." -ForegroundColor Cyan
Push-Location $projectRoot
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo release build failed with exit code $LASTEXITCODE"
    }

    $exePath = Join-Path $projectRoot "target\release\slowspot.exe"
    if (-not (Test-Path $exePath)) {
        throw "Release binary not found at $exePath"
    }

    $targetOutDir = Join-Path $projectRoot $OutDir
    if (-not (Test-Path $targetOutDir)) {
        New-Item -ItemType Directory -Path $targetOutDir -Force | Out-Null
    }

    # 2. Run at 1000x680
    Write-Host "Capturing screenshots at 1000x680..." -ForegroundColor Cyan
    $p1 = Start-Process -FilePath $exePath -ArgumentList @("--demo", "--screenshot", $targetOutDir, "--size", "1000x680") -Wait -PassThru -NoNewWindow
    if ($p1.ExitCode -ne 0) {
        throw "Screenshot capture failed at 1000x680 with exit code $($p1.ExitCode)"
    }

    # 3. Run at 760x520
    Write-Host "Capturing screenshots at 760x520..." -ForegroundColor Cyan
    $p2 = Start-Process -FilePath $exePath -ArgumentList @("--demo", "--screenshot", $targetOutDir, "--size", "760x520") -Wait -PassThru -NoNewWindow
    if ($p2.ExitCode -ne 0) {
        throw "Screenshot capture failed at 760x520 with exit code $($p2.ExitCode)"
    }

    # 4. Convert BMPs to PNG for easy viewing
    Add-Type -AssemblyName System.Drawing
    Get-ChildItem -Path $targetOutDir -Filter "*.bmp" | ForEach-Object {
        $bmp = [System.Drawing.Image]::FromFile($_.FullName)
        $pngPath = [System.IO.Path]::ChangeExtension($_.FullName, ".png")
        $bmp.Save($pngPath, [System.Drawing.Imaging.ImageFormat]::Png)
        $bmp.Dispose()
    }

    # 5. Print file list
    Write-Host "`nGenerated screenshots in ${targetOutDir}:" -ForegroundColor Green
    Get-ChildItem -Path $targetOutDir -Include "*.bmp", "*.png" | Select-Object Name, Length, LastWriteTime | Format-Table -AutoSize
} finally {
    Pop-Location
}
