[CmdletBinding()]
param([string]$ExperimentDir)
$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
if (-not $ExperimentDir) {
    $ExperimentDir = Join-Path $env:TEMP ('xemnas-agent-loop-' + [guid]::NewGuid().ToString('N'))
}
if (Test-Path -LiteralPath $ExperimentDir) {
    throw 'Use a new experiment directory; existing data is preserved.'
}
New-Item -ItemType Directory -Path (Join-Path $ExperimentDir 'scenario/src') -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $ExperimentDir 'harness/src') -Force | Out-Null
Set-Content -LiteralPath (Join-Path $ExperimentDir 'scenario/README.md') -Encoding utf8 -Value "# RelayDesk`nAplicativo desktop para manter uma fila local de entregas e pesquisar registros."
Set-Content -LiteralPath (Join-Path $ExperimentDir 'scenario/src/storage.rs') -Encoding utf8 -Value 'pub const DATABASE: &str = "sqlite";'
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'runner.rs') -Destination (Join-Path $ExperimentDir 'harness/src/main.rs')
$crateRoot = $repoRoot.Replace('\', '/')
$manifest = @"
[package]
name = "xemnas-agent-experiment"
version = "0.1.0"
edition = "2021"
[dependencies]
application = { path = "$crateRoot/crates/application" }
storage-sqlite = { path = "$crateRoot/crates/storage-sqlite" }
local-api = { path = "$crateRoot/crates/local-api" }
[profile.dev]
opt-level = 1
"@
$manifestPath = Join-Path $ExperimentDir 'harness/Cargo.toml'
Set-Content -LiteralPath $manifestPath -Encoding utf8 -Value $manifest
# Adapt only the copied lockfile for the external experiment package.
Copy-Item -LiteralPath (Join-Path $repoRoot 'Cargo.lock') -Destination (Join-Path $ExperimentDir 'harness/Cargo.lock')
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
& cargo build --offline --manifest-path $manifestPath --target-dir (Join-Path $repoRoot 'target')
if ($LASTEXITCODE -ne 0) { throw 'Experiment build failed; artifacts were preserved.' }
Write-Output "ExperimentDir=$ExperimentDir"
Write-Output "Binary=$(Join-Path $repoRoot 'target/debug/xemnas-agent-experiment.exe')"
Write-Output 'Run the binary with ExperimentDir as its first argument in an interactive terminal.'
Write-Output 'Press Enter to stop; use the second argument reopen to restart without seeding again.'
