# scripts/package.ps1
# SlowSpot standalone release packaging script.
# Builds release binary, creates clean dist directory, packages portable zip, and prints SHA256 checksums.

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$scriptDir = $PSScriptRoot
$projectRoot = Split-Path -Parent $scriptDir
Set-Location $projectRoot

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host " SlowSpot Release Packaging" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

# 1. Compile optimized release binary
Write-Host "[1/4] Building release binary with Cargo..." -ForegroundColor Yellow
$cargo = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $cargo) {
    throw "Cargo command not found in PATH. Please ensure the Rust toolchain is installed."
}

& cargo build --release
if ($LASTEXITCODE -ne 0) {
    throw "Cargo release build failed with exit code $LASTEXITCODE."
}

$releaseExe = Join-Path $projectRoot "target\release\slowspot.exe"
if (-not (Test-Path $releaseExe)) {
    throw "Expected binary was not found at: $releaseExe"
}

# 2. Clean and recreate dist directory
Write-Host "[2/4] Preparing clean dist directory..." -ForegroundColor Yellow
$distDir = Join-Path $projectRoot "dist"
if (Test-Path $distDir) {
    Remove-Item -Path $distDir -Recurse -Force
}
New-Item -ItemType Directory -Path $distDir -Force | Out-Null

$distExe = Join-Path $distDir "slowspot.exe"
Copy-Item -Path $releaseExe -Destination $distExe -Force

# 3. Create portable zip archive
Write-Host "[3/4] Assembling portable zip package..." -ForegroundColor Yellow
$stagingDir = Join-Path $distDir "_staging"
if (Test-Path $stagingDir) {
    Remove-Item -Path $stagingDir -Recurse -Force
}
New-Item -ItemType Directory -Path $stagingDir -Force | Out-Null

# Copy executable into staging
Copy-Item -Path $distExe -Destination (Join-Path $stagingDir "slowspot.exe") -Force

# Create short user guide in staging
$shortReadmePath = Join-Path $stagingDir "README.txt"
$shortReadmeContent = @"
======================================================================
SlowSpot - Windows Performance Bottleneck Monitor (Portable Edition)
======================================================================

SlowSpot la ung dung desktop nhe, doc lap giup giam sat tien trinh
va chi ro thu pham lam cham may tinh (CPU, RAM, Disk I/O).

1. HUONG DAN SU DUNG:
   - Chay truc tiep file 'slowspot.exe'.
   - Khong can cai dat, khong ghi Registry, khong can Visual C++ Redistributable.
   - Khi may bi cham, SlowSpot se tu dong danh gia va hien the 'Thu pham'
     o dau giao dien de ban dong an toan hoac tat cuong buc.

2. CHE DO MO PHONG (DEMO MODE):
   - Chay bang dong lenh:
     slowspot.exe --demo
   - Che do demo mo phong cac tinh huong nghen CPU, nghen Dia, Can RAM
     ma khong anh huong den tien trinh that tren he thong.

3. YEU CAU HE THONG:
   - Windows 10 / Windows 11 (64-bit).
   - Driver card do hoa ho tro OpenGL 2.0 tro len.

4. BAN QUYEN & GIAY PHEP:
   - SlowSpot: license not yet decided by the project owner.
   - Font Noto Sans: SIL Open Font License 1.1 (xem file OFL.txt).
"@
Set-Content -Path $shortReadmePath -Value $shortReadmeContent -Encoding UTF8

# Copy font license
$fontLicenseSrc = Join-Path $projectRoot "assets\fonts\OFL.txt"
if (Test-Path $fontLicenseSrc) {
    $licenseSubdir = Join-Path $stagingDir "assets\fonts"
    New-Item -ItemType Directory -Path $licenseSubdir -Force | Out-Null
    Copy-Item -Path $fontLicenseSrc -Destination (Join-Path $licenseSubdir "OFL.txt") -Force
    Copy-Item -Path $fontLicenseSrc -Destination (Join-Path $stagingDir "OFL.txt") -Force
} else {
    Write-Warning "assets\fonts\OFL.txt not found. Font license was omitted from zip."
}

# Compress into zip
$distZip = Join-Path $distDir "slowspot-portable.zip"
if (Test-Path $distZip) {
    Remove-Item -Path $distZip -Force
}
Compress-Archive -Path (Join-Path $stagingDir "*") -DestinationPath $distZip -Force

# Clean up staging directory
Remove-Item -Path $stagingDir -Recurse -Force

# 4. Compute file sizes and SHA256 hashes
Write-Host "[4/4] Computing file checksums..." -ForegroundColor Yellow
$exeItem = Get-Item $distExe
$zipItem = Get-Item $distZip

$exeHash = (Get-FileHash -Path $distExe -Algorithm SHA256).Hash
$zipHash = (Get-FileHash -Path $distZip -Algorithm SHA256).Hash

Write-Host ""
Write-Host "============================================================" -ForegroundColor Green
Write-Host " SlowSpot Packaging Completed Successfully" -ForegroundColor Green
Write-Host "============================================================" -ForegroundColor Green
Write-Host ("Binary : {0}" -f $distExe)
Write-Host ("Size   : {0:N0} bytes ({1:N2} MB)" -f $exeItem.Length, ($exeItem.Length / 1MB))
Write-Host ("SHA256 : {0}" -f $exeHash)
Write-Host ""
Write-Host ("Zip    : {0}" -f $distZip)
Write-Host ("Size   : {0:N0} bytes ({1:N2} MB)" -f $zipItem.Length, ($zipItem.Length / 1MB))
Write-Host ("SHA256 : {0}" -f $zipHash)
Write-Host "============================================================" -ForegroundColor Green

[PSCustomObject]@{
    Artifact = "slowspot.exe"
    Path     = $distExe
    SizeBytes = $exeItem.Length
    SizeMB   = [math]::Round($exeItem.Length / 1MB, 2)
    SHA256   = $exeHash
},
[PSCustomObject]@{
    Artifact = "slowspot-portable.zip"
    Path     = $distZip
    SizeBytes = $zipItem.Length
    SizeMB   = [math]::Round($zipItem.Length / 1MB, 2)
    SHA256   = $zipHash
}
