[CmdletBinding()]
param(
    [string]$WorkDirectory = 'D:\xemnas-semantic-bench',
    [switch]$SkipBuild
)
$ErrorActionPreference = 'Stop'
$source = $PSScriptRoot
New-Item -ItemType Directory -Force -Path $WorkDirectory | Out-Null
$work = (Resolve-Path -LiteralPath $WorkDirectory).Path
New-Item -ItemType Directory -Force -Path (Join-Path $work 'native-target') | Out-Null
$runName = 'run-' + (Get-Date -Format 'yyyyMMdd-HHmmss')
& (Join-Path $source 'prepare-datasets.ps1') -OutputDirectory (Join-Path $work 'datasets')
$ortDirectory = Join-Path $work 'ort'
New-Item -ItemType Directory -Force -Path $ortDirectory | Out-Null
$ortArchive = Join-Path $ortDirectory 'onnxruntime-linux-x64-1.30.0.tgz'
if (-not (Test-Path -LiteralPath $ortArchive)) {
    Invoke-WebRequest -Uri 'https://github.com/microsoft/onnxruntime/releases/download/v1.30.0/onnxruntime-linux-x64-1.30.0.tgz' -OutFile $ortArchive
}
$expectedHash = 'a5ed5a3cac51fbb2e90da632ae43d19212faaa20e76484e62bcb7c23ddb3b3fd'
if ((Get-FileHash -LiteralPath $ortArchive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expectedHash) {
    throw 'Official ONNX Runtime archive checksum mismatch.'
}
if (-not $SkipBuild) {
    docker build -t xemnas-semantic-bench:local $source
    if ($LASTEXITCODE -ne 0) { throw 'Benchmark image build failed.' }
    docker run --rm --mount "type=bind,source=$work,target=/bench" `
        --mount "type=bind,source=$source,target=/source,readonly" `
        --mount 'type=volume,source=xemnas-semantic-cargo,target=/cargo' `
        xemnas-semantic-bench:local sh /source/prepare-lancedb.sh
    if ($LASTEXITCODE -ne 0) { throw 'Isolated LanceDB source preparation failed.' }
    docker run --rm --mount "type=bind,source=$work,target=/bench" `
        --mount "type=bind,source=$source,target=/source,readonly" `
        --mount 'type=volume,source=xemnas-semantic-cargo,target=/cargo' -e CARGO_HOME=/cargo `
        --mount "type=bind,source=$work\native-target,target=/build" -e CARGO_TARGET_DIR=/build `
        xemnas-semantic-bench:local cargo build --locked --release --features lance -j 4 `
        --config 'patch.crates-io.lancedb.path="/bench/lancedb-patched"'
    if ($LASTEXITCODE -ne 0) { throw 'Rust benchmark build failed.' }
}
docker run --rm --memory 8g --memory-swap 8g --cpus 4 `
    --mount "type=bind,source=$work,target=/bench" `
    --mount "type=bind,source=$source,target=/source,readonly" `
    --mount "type=bind,source=$work\native-target,target=/build,readonly" `
    -e ORT_DYLIB_PATH=/bench/ort/onnxruntime-linux-x64-1.30.0/lib/libonnxruntime.so.1.30.0 `
    xemnas-semantic-bench:local sh -c 'sh /source/prepare-models.sh && tar -xzf /bench/ort/onnxruntime-linux-x64-1.30.0.tgz -C /bench/ort && /build/release/semantic-bench prepare e5 4 /bench/models /bench/prefetch-e5 && /build/release/semantic-bench prepare gemma-q4 4 /bench/models /bench/prefetch-gemma'
if ($LASTEXITCODE -ne 0) { throw 'Model preparation failed.' }

$hostInfo = [ordered]@{
    captured_at = (Get-Date -Format o)
    os = (Get-CimInstance Win32_OperatingSystem | Select-Object Caption, Version, BuildNumber)
    cpu = (Get-CimInstance Win32_Processor | Select-Object Name, NumberOfCores, NumberOfLogicalProcessors)
    physical_ram_bytes = (Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory
    power_plan = (powercfg /getactivescheme | Out-String).Trim()
    background_container_count = @(docker ps -q).Count
    benchmark_image_id = (docker image inspect xemnas-semantic-bench:local --format '{{.Id}}')
    isolation = 'Docker Linux/WSL2, 8 GiB memory and no swap, quota 4 CPUs; other host services remain active.'
}
$hostInfo | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $work "$runName-host.json") -Encoding utf8NoBOM
docker run --rm --name xemnas-semantic-measure --network none --memory 8g --memory-swap 8g --cpus 4 `
    --mount "type=bind,source=$work,target=/bench" `
    --mount "type=bind,source=$source,target=/source,readonly" `
    --mount 'type=volume,source=xemnas-semantic-scratch,target=/scratch' `
    --mount "type=bind,source=$work\native-target,target=/build,readonly" `
    xemnas-semantic-bench:local sh /source/run.sh "/bench/$runName"
if ($LASTEXITCODE -ne 0) { throw "Benchmark failed. Inspect $work\$runName logs." }
Copy-Item -LiteralPath (Join-Path $work "$runName-host.json") -Destination (Join-Path $work "$runName/host.json")
Write-Output "Results: $work\$runName"
