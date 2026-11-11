param(
    [Parameter(Mandatory = $true)][string]$Label
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $projectRoot
$targetPath = Join-Path $projectRoot "target/speed-$Label"
$reportPath = Join-Path $projectRoot "temp/build-speed/$Label"
if (Test-Path -LiteralPath $targetPath) {
    throw "Use a new label: $targetPath already exists."
}
New-Item -ItemType Directory -Path $reportPath -Force | Out-Null
Copy-Item -LiteralPath Cargo.toml,Cargo.lock -Destination $reportPath
$toolchain = (& rustc -Vv) -join "`n"
$results = @()
foreach ($phase in @('cold', 'unchanged')) {
    $logPath = Join-Path $reportPath "$phase.log"
    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    # Native stderr contains Cargo progress, not PowerShell failures.
    $ErrorActionPreference = 'Continue'
    & cargo build --locked --bin redox --target-dir $targetPath --timings *> $logPath
    $code = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    $timer.Stop()
    $results += [pscustomobject]@{
        phase = $phase
        seconds = [math]::Round($timer.Elapsed.TotalSeconds, 3)
        exit_code = $code
    }
    [pscustomobject]@{
        label = $Label
        toolchain = $toolchain
        command = 'cargo build --locked --bin redox --target-dir <isolated target> --timings'
        results = $results
    } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $reportPath 'results.json')
    $results[-1] | Format-List
    if ($code -ne 0) {
        Get-Content -LiteralPath $logPath -Tail 40
        exit $code
    }
}
