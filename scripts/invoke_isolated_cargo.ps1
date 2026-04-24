# Reason: standardize Windows-local Cargo verification to avoid reused target and exe-lock pollution.
# Purpose: run Cargo with a fresh isolated target directory and an optional captured log for truthful verification.
# Time: 2026-04-24
param(
    [Parameter(Mandatory = $true)]
    [string]$CargoCommand,

    [string[]]$CargoArguments = @(),

    [string]$RunLabel = "cargo",

    [string]$WorkspaceRoot = (Get-Location).Path,

    [string]$ArtifactRoot = ".verification",

    [switch]$NoLog
)

$ErrorActionPreference = "Stop"

function Get-SafeLabel {
    param(
        [string]$Value
    )

    if ([string]::IsNullOrWhiteSpace($Value)) {
        return "cargo"
    }

    $safe = $Value -replace "[^A-Za-z0-9._-]", "_"
    $safe = $safe.Trim("_")
    if ([string]::IsNullOrWhiteSpace($safe)) {
        return "cargo"
    }

    return $safe
}

function Quote-CmdArgument {
    param(
        [string]$Value
    )

    if ($null -eq $Value) {
        return '""'
    }

    return '"' + ($Value -replace '"', '\"') + '"'
}

$resolvedWorkspaceRoot = (Resolve-Path -LiteralPath $WorkspaceRoot).Path
$safeLabel = Get-SafeLabel -Value $RunLabel
$timestamp = Get-Date -Format "yyyyMMdd_HHmmss_fff"
$runId = "{0}_{1}_{2}" -f $safeLabel, $timestamp, $PID
$resolvedArtifactRoot = Join-Path $resolvedWorkspaceRoot $ArtifactRoot
$targetRoot = Join-Path $resolvedArtifactRoot "cargo-targets"
$logRoot = Join-Path $resolvedArtifactRoot "logs"
$targetDir = Join-Path $targetRoot $runId
$logPath = Join-Path $logRoot ("{0}.log" -f $runId)
$stdoutPath = Join-Path $logRoot ("{0}.stdout.log" -f $runId)
$stderrPath = Join-Path $logRoot ("{0}.stderr.log" -f $runId)

New-Item -ItemType Directory -Force -Path $targetRoot | Out-Null
if (-not $NoLog) {
    New-Item -ItemType Directory -Force -Path $logRoot | Out-Null
}

$originalTargetDir = $env:CARGO_TARGET_DIR

try {
    $env:CARGO_TARGET_DIR = $targetDir

    Write-Host ("[isolated-cargo] workspace={0}" -f $resolvedWorkspaceRoot)
    Write-Host ("[isolated-cargo] target={0}" -f $targetDir)
    if (-not $NoLog) {
        Write-Host ("[isolated-cargo] log={0}" -f $logPath)
    }
    $cargoArgs = @($CargoCommand) + $CargoArguments

    Write-Host ("[isolated-cargo] cargo {0}" -f ($cargoArgs -join " "))

    $quotedArgs = $cargoArgs | ForEach-Object { Quote-CmdArgument -Value $_ }
    $cargoCommandLine = $quotedArgs -join " "
    $process = Start-Process `
        -FilePath "cargo.exe" `
        -ArgumentList $cargoCommandLine `
        -WorkingDirectory $resolvedWorkspaceRoot `
        -NoNewWindow `
        -PassThru `
        -Wait `
        -RedirectStandardOutput $stdoutPath `
        -RedirectStandardError $stderrPath

    $exitCode = $process.ExitCode
    if ($null -eq $exitCode) {
        $exitCode = 0
    }

    if (Test-Path $stdoutPath) {
        Get-Content -Path $stdoutPath
    }

    if (Test-Path $stderrPath) {
        Get-Content -Path $stderrPath
    }

    if (-not $NoLog) {
        $combinedLog = @()
        if (Test-Path $stdoutPath) {
            $combinedLog += Get-Content -Path $stdoutPath
        }
        if (Test-Path $stderrPath) {
            $combinedLog += Get-Content -Path $stderrPath
        }
        $combinedLog | Set-Content -Path $logPath
    }

    Write-Host ("[isolated-cargo] exit_code={0}" -f $exitCode)
    exit $exitCode
}
finally {
    if ($null -eq $originalTargetDir) {
        Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    } else {
        $env:CARGO_TARGET_DIR = $originalTargetDir
    }
}
