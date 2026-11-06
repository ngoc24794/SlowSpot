# scripts/verify.ps1
# SlowSpot standalone verification script.
# Validates portable binary integrity:
# 1. Dependency check (no vcruntime140*, msvcp140*, or api-ms-win-crt-*)
# 2. Isolated folder execution (--demo, alive after 8s, title "SlowSpot", ~10s CPU/RAM measurement, 0 residual files)
# 3. Clean termination of target PID and structured PASS/FAIL reporting.

[CmdletBinding()]
param(
    [string]$ExePath = ""
)

$ErrorActionPreference = "Stop"

$scriptDir = $PSScriptRoot
$projectRoot = Split-Path -Parent $scriptDir

# Resolve target executable path
if ([string]::IsNullOrWhiteSpace($ExePath)) {
    $distExe = Join-Path $projectRoot "dist\slowspot.exe"
    $targetExe = Join-Path $projectRoot "target\release\slowspot.exe"
    if (Test-Path $distExe) {
        $ExePath = $distExe
    } elseif (Test-Path $targetExe) {
        $ExePath = $targetExe
    } else {
        throw "Target executable not found. Please run scripts/package.ps1 first, or pass -ExePath."
    }
}

$ExePath = (Resolve-Path $ExePath).Path
if (-not (Test-Path $ExePath)) {
    throw "Executable does not exist at path: $ExePath"
}

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host " SlowSpot Portable Verification" -ForegroundColor Cyan
Write-Host " Target : $ExePath" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

$results = [System.Collections.Generic.List[PSCustomObject]]::new()
$allPassed = $true

function Add-Result {
    param(
        [string]$Item,
        [bool]$Passed,
        [string]$Details
    )
    $script:results.Add([PSCustomObject]@{
        Item    = $Item
        Status  = if ($Passed) { "PASS" } else { "FAIL" }
        Details = $Details
    })
    if (-not $Passed) {
        $script:allPassed = $false
    }
}

# -----------------------------------------------------------------------------
# CHECK 1: DLL Dependency Verification (dumpbin with pure PowerShell PE fallback)
# -----------------------------------------------------------------------------
Write-Host "`n[Check 1/6] Scanning imported DLL dependencies..." -ForegroundColor Yellow

$importedDlls = @()
$dumpbinPath = $null

# Attempt 1: dumpbin in PATH
$dumpbinCmd = Get-Command dumpbin.exe -ErrorAction SilentlyContinue
if ($dumpbinCmd) {
    $dumpbinPath = $dumpbinCmd.Source
}

# Attempt 2: vswhere
if (-not $dumpbinPath) {
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path $vswhere) {
        $vsDumpbin = & $vswhere -latest -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -find **\dumpbin.exe | Select-Object -First 1
        if ($vsDumpbin -and (Test-Path $vsDumpbin)) {
            $dumpbinPath = $vsDumpbin
        }
    }
}

if ($dumpbinPath) {
    Write-Host "  Using dumpbin: $dumpbinPath" -ForegroundColor DarkGray
    $output = & $dumpbinPath /dependents $ExePath
    $capture = $false
    foreach ($line in $output) {
        $trimmed = $line.Trim()
        if ($trimmed -eq "Image has the following dependencies:") {
            $capture = $true
            continue
        }
        if ($capture) {
            if ($trimmed.StartsWith("Summary")) {
                break
            }
            if ([string]::IsNullOrWhiteSpace($trimmed)) {
                continue
            }
            if ($trimmed.EndsWith(".dll", [System.StringComparison]::OrdinalIgnoreCase)) {
                $importedDlls += $trimmed.ToLowerInvariant()
            }
        }
    }
} else {
    Write-Host "  dumpbin not found in PATH or VS Installer; falling back to native PE import table parser..." -ForegroundColor DarkYellow
    try {
        $bytes = [System.IO.File]::ReadAllBytes($ExePath)
        $e_lfanew = [BitConverter]::ToInt32($bytes, 0x3c)
        $sizeOfOptHeader = [BitConverter]::ToUInt16($bytes, $e_lfanew + 20)
        $numSections = [BitConverter]::ToUInt16($bytes, $e_lfanew + 6)
        $optHeader = $e_lfanew + 24
        $importRva = [BitConverter]::ToUInt32($bytes, $optHeader + 112 + 8)
        $secHeaders = $optHeader + $sizeOfOptHeader

        function LocalRvaToOffset($rva, $bytesRef, $secStart, $nSections) {
            for ($i = 0; $i -lt $nSections; $i++) {
                $sec = $secStart + ($i * 40)
                $vSize = [BitConverter]::ToUInt32($bytesRef, $sec + 8)
                $vAddr = [BitConverter]::ToUInt32($bytesRef, $sec + 12)
                $rawAddr = [BitConverter]::ToUInt32($bytesRef, $sec + 20)
                if ($rva -ge $vAddr -and $rva -lt ($vAddr + $vSize)) {
                    return ($rva - $vAddr) + $rawAddr
                }
            }
            return $null
        }

        $descOffset = LocalRvaToOffset $importRva $bytes $secHeaders $numSections
        if ($descOffset) {
            while ($true) {
                $nameRva = [BitConverter]::ToUInt32($bytes, $descOffset + 12)
                if ($nameRva -eq 0) { break }
                $nameOffset = LocalRvaToOffset $nameRva $bytes $secHeaders $numSections
                if (-not $nameOffset) { break }
                $nameBytes = @()
                $cur = $nameOffset
                while ($cur -lt $bytes.Length -and $bytes[$cur] -ne 0) {
                    $nameBytes += $bytes[$cur]
                    $cur++
                }
                $dllName = [System.Text.Encoding]::ASCII.GetString([byte[]]$nameBytes)
                $importedDlls += $dllName.ToLowerInvariant()
                $descOffset += 20
            }
        }
    } catch {
        Write-Warning "Failed to parse PE import table: $_"
    }
}

# Unique sorted DLLs
$importedDlls = $importedDlls | Sort-Object -Unique

Write-Host "  Imported DLLs ($($importedDlls.Count)):" -ForegroundColor DarkGray
foreach ($d in $importedDlls) {
    Write-Host "    - $d" -ForegroundColor DarkGray
}

$prohibitedDlls = $importedDlls | Where-Object {
    $_ -like "vcruntime140*" -or
    $_ -like "msvcp140*" -or
    $_ -like "api-ms-win-crt-*"
}

if ($importedDlls.Count -eq 0) {
    Add-Result "DLL Dependency Scan" $false "Failed to enumerate imported DLLs"
} elseif ($prohibitedDlls.Count -gt 0) {
    Add-Result "Static CRT Independence" $false "Found prohibited CRT runtime DLLs: $($prohibitedDlls -join ', ')"
} else {
    Add-Result "Static CRT Independence" $true "No VCRUNTIME/MSVCP/api-ms-win-crt-* dependencies. Total system DLLs: $($importedDlls.Count)"
}

# -----------------------------------------------------------------------------
# CHECK 2 - 5: Isolated Clean Folder Execution, Title, Metrics, Residual Files
# -----------------------------------------------------------------------------
$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("slowspot_verify_" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tempDir -Force | Out-Null
Write-Host "`nCreated isolated sandbox directory: $tempDir" -ForegroundColor DarkGray

$targetPid = $null
$proc = $null

try {
    $tempExe = Join-Path $tempDir "slowspot.exe"
    Copy-Item -Path $ExePath -Destination $tempExe -Force

    Write-Host "[Check 2/6] Starting isolated process with --demo and waiting 8 seconds..." -ForegroundColor Yellow
    $proc = Start-Process -FilePath $tempExe -ArgumentList "--demo" -WorkingDirectory $tempDir -PassThru
    $targetPid = $proc.Id
    Write-Host "  Started PID: $targetPid" -ForegroundColor DarkGray

    Start-Sleep -Seconds 8
    $proc.Refresh()

    # Verify process is still alive
    if ($proc.HasExited) {
        Add-Result "Process Survival (8s)" $false "Process exited prematurely with exit code $($proc.ExitCode)"
    } else {
        Add-Result "Process Survival (8s)" $true "Process is alive after 8 seconds (PID $targetPid)"
    }

    # Verify MainWindowTitle is "SlowSpot"
    Write-Host "[Check 3/6] Inspecting MainWindowTitle..." -ForegroundColor Yellow
    $title = $proc.MainWindowTitle
    if ([string]::IsNullOrWhiteSpace($title)) {
        # Retry once after 500ms
        Start-Sleep -Milliseconds 500
        $proc.Refresh()
        $title = $proc.MainWindowTitle
    }
    Write-Host "  MainWindowTitle: '$title'" -ForegroundColor DarkGray

    if ($title -eq "SlowSpot") {
        Add-Result "MainWindowTitle Check" $true "Window title matches expected: '$title'"
    } else {
        Add-Result "MainWindowTitle Check" $false "Expected MainWindowTitle 'SlowSpot', got: '$title'"
    }

    # Measure CPU% and Private Memory over ~10 seconds
    Write-Host "[Check 4/6] Measuring CPU and Memory consumption over ~10 seconds..." -ForegroundColor Yellow
    $proc.Refresh()
    $initialCpu = $proc.TotalProcessorTime.TotalSeconds
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $privateMemSamples = @()
    $workingSetSamples = @()

    for ($i = 1; $i -le 10; $i++) {
        Start-Sleep -Seconds 1
        $proc.Refresh()
        if ($proc.HasExited) {
            throw "Process exited prematurely during the 10-second measurement window."
        }
        $privateMemSamples += $proc.PrivateMemorySize64
        $workingSetSamples += $proc.WorkingSet64
    }
    $sw.Stop()
    $proc.Refresh()

    $finalCpu = $proc.TotalProcessorTime.TotalSeconds
    $elapsedSeconds = $sw.Elapsed.TotalSeconds
    $coreCount = [Environment]::ProcessorCount
    $cpuDelta = [Math]::Max(0.0, ($finalCpu - $initialCpu))
    $avgCpuPercent = ($cpuDelta / ($elapsedSeconds * $coreCount)) * 100.0
    $avgPrivateMemMB = ($privateMemSamples | Measure-Object -Average).Average / 1MB
    $avgWorkingSetMB = ($workingSetSamples | Measure-Object -Average).Average / 1MB

    Write-Host ("  Average CPU%          : {0:N2}% (across {1} logical cores)" -f $avgCpuPercent, $coreCount) -ForegroundColor DarkGray
    Write-Host ("  Average Private Memory: {0:N2} MB" -f $avgPrivateMemMB) -ForegroundColor DarkGray
    Write-Host ("  Average Working Set   : {0:N2} MB" -f $avgWorkingSetMB) -ForegroundColor DarkGray

    Add-Result "Resource Metrics" $true ("CPU: {0:N2}%, Private Memory: {1:N2} MB, Working Set: {2:N2} MB (10s avg)" -f $avgCpuPercent, $avgPrivateMemMB, $avgWorkingSetMB)

    # Check for residual files in temporary directory
    Write-Host "[Check 5/6] Checking for residual files generated in isolated folder..." -ForegroundColor Yellow
    $allFiles = Get-ChildItem -Path $tempDir -Recurse
    $extraFiles = $allFiles | Where-Object { $_.FullName -ne $tempExe }
    if ($extraFiles.Count -eq 0) {
        Add-Result "Zero Residual Files" $true "No unexpected files generated in sandbox folder"
    } else {
        $extraNames = ($extraFiles | Select-Object -ExpandProperty Name) -join ", "
        Add-Result "Zero Residual Files" $false "Found unexpected residual files: $extraNames"
    }

} catch {
    Add-Result "Execution Verification" $false "Exception encountered: $($_.Exception.Message)"
} finally {
    # -------------------------------------------------------------------------
    # CHECK 6: Stop exact target PID
    # -------------------------------------------------------------------------
    Write-Host "[Check 6/6] Terminating target process PID $targetPid..." -ForegroundColor Yellow
    if ($targetPid) {
        try {
            $p = Get-Process -Id $targetPid -ErrorAction SilentlyContinue
            if ($p -and -not $p.HasExited) {
                Stop-Process -Id $targetPid -Force
                $p.WaitForExit(5000) | Out-Null
                Add-Result "Process Cleanup" $true "PID $targetPid terminated cleanly"
            } else {
                Add-Result "Process Cleanup" $true "PID $targetPid had already stopped"
            }
        } catch {
            Add-Result "Process Cleanup" $false "Error terminating PID ${targetPid}: $($_.Exception.Message)"
        }
    }

    # Clean up sandbox folder
    if (Test-Path $tempDir) {
        # Give OS a brief moment to release file handles if any
        Start-Sleep -Milliseconds 500
        Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# -----------------------------------------------------------------------------
# SUMMARY & EXIT CODE
# -----------------------------------------------------------------------------
Write-Host "`n============================================================" -ForegroundColor Cyan
Write-Host " Verification Summary Report" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

foreach ($r in $results) {
    $color = if ($r.Status -eq "PASS") { "Green" } else { "Red" }
    Write-Host ("[{0}] {1,-28} : {2}" -f $r.Status, $r.Item, $r.Details) -ForegroundColor $color
}
Write-Host "============================================================" -ForegroundColor Cyan

if ($allPassed) {
    Write-Host "`n>>> OVERALL RESULT: ALL CHECKS PASSED <<<`n" -ForegroundColor Green
    exit 0
} else {
    Write-Host "`n>>> OVERALL RESULT: VERIFICATION FAILED <<<`n" -ForegroundColor Red
    exit 1
}
