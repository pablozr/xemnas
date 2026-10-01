[CmdletBinding()]
param([Parameter(Mandatory)][string]$RunDirectory)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $RunDirectory).Path
function Read-Result([string]$relative) {
    Get-Content -LiteralPath (Join-Path $root $relative) -Raw | ConvertFrom-Json
}
function Assert-Samples($stats, [int]$expected) {
    if ($stats.n -ne $expected -or $stats.samples_ms.Count -ne $expected) {
        throw "Wrong sample count: expected $expected"
    }
    foreach ($sample in $stats.samples_ms) {
        if (-not [double]::IsFinite($sample) -or $sample -lt 0) { throw 'Invalid timing' }
    }
}
function Median($values) {
    $ordered = @($values | Sort-Object)
    $ordered[[int][Math]::Floor($ordered.Count / 2)]
}
function Summary($values) {
    $numbers = @($values)
    [ordered]@{ median = (Median $numbers); min = ($numbers | Measure-Object -Minimum).Minimum;
        max = ($numbers | Measure-Object -Maximum).Maximum; runs = $numbers }
}
function Peak-MiB($memory) {
    [double]($memory.VmHWM -split ' ')[0] / 1024
}
function Assert-Quality($quality, [int]$queryCount, [int]$documentCount) {
    if ($quality.queries -ne $queryCount -or $quality.rankings.Count -ne $queryCount) {
        throw 'Wrong quality ranking count'
    }
    foreach ($metric in @('recall_at_10', 'mrr_at_10', 'ndcg_at_10', 'top1_relevant_rate')) {
        if (-not [double]::IsFinite($quality.$metric) -or $quality.$metric -lt 0 -or $quality.$metric -gt 1) {
            throw "Invalid quality metric: $metric"
        }
    }
    foreach ($ranking in $quality.rankings) {
        if (@($ranking | Sort-Object -Unique).Count -ne $ranking.Count) { throw 'Duplicate quality hits' }
        if (@($ranking | Where-Object { $_ -lt 0 -or $_ -ge $documentCount }).Count) { throw 'Invalid quality ID' }
    }
}
$embedding = @()
$storage = @()
$public = @()
$agreement = @()
foreach ($model in @('e5', 'gemma-q4')) {
    $runs = @(1..3 | ForEach-Object { Read-Result "$model-t4-r$_/embedding.json" })
    foreach ($run in $runs) {
        Assert-Samples $run.warm_query 120
        Assert-Quality $run.quality 24 48
        if ($run.query_tokens.Count -ne 24 -or $run.document_tokens.Count -ne 48) {
            throw 'Fixture shape changed'
        }
        if ($run.memory.cgroup_swap_current_bytes -ne '0') { throw 'Swap used' }
        foreach ($profile in $run.length_profiles) { Assert-Samples $profile.latency 20 }
    }
    $single = Read-Result "$model-t1/embedding.json"
    Assert-Samples $single.warm_query 120
    $embedding += [ordered]@{
        model = $model; dimension = $runs[0].dimension
        initialization_ms = (Summary $runs.initialization_ms)
        first_query_ms = (Summary $runs.first_query_ms)
        warm_p50_ms = (Summary $runs.warm_query.p50_ms)
        warm_p95_ms = (Summary $runs.warm_query.p95_ms)
        peak_mib = (Summary @($runs | ForEach-Object { Peak-MiB $_.memory }))
        short_query_peak_mib = (Summary @($runs | ForEach-Object { Peak-MiB $_.memory_after_short_queries }))
        corpus48_batch8_mean_ms = (Summary $runs.batch_8_corpus.mean_ms)
        single_thread = $single
        quality = $runs[0].quality
        length_profiles = @(foreach ($target in @(128, 256, 512)) {
            $profiles = @($runs.length_profiles | Where-Object target_tokens -eq $target)
            [ordered]@{ target = $target; actual_tokens = $profiles[0].actual_tokens;
                p50_ms = (Summary $profiles.latency.p50_ms); p95_ms = (Summary $profiles.latency.p95_ms) }
        })
    }
    foreach ($rows in @(48, 1000, 10000, 100000)) {
        foreach ($engine in @('sqlite', 'lance')) {
            $cases = @(1..3 | ForEach-Object { Read-Result "$engine-$model-$rows-r$_/storage.json" })
            foreach ($case in $cases) {
                foreach ($stage in @('vector', 'fts', 'hybrid_rrf')) { Assert-Samples $case.$stage 240 }
                if ($case.rows -ne $rows -or $case.dimension -ne $runs[0].dimension) { throw 'Wrong storage case' }
                if ($rows -eq 48) {
                    foreach ($kind in @('vector', 'fts', 'hybrid')) { Assert-Quality $case.quality.$kind 24 48 }
                }
                foreach ($ranking in $case.first_vector_rankings) {
                    if ($ranking.Count -ne 10 -or @($ranking | Sort-Object -Unique).Count -ne 10) {
                        throw 'Duplicate or missing vector hits'
                    }
                    if (@($ranking | Where-Object { $_ -lt 0 -or $_ -ge $rows }).Count) { throw 'Invalid ID' }
                }
                foreach ($ann in $case.ann) {
                    Assert-Samples $ann.latency 240
                    if ($ann.recall_at_10_vs_exact -lt 0 -or $ann.recall_at_10_vs_exact -gt 1) { throw 'Invalid ANN recall' }
                }
            }
            $pipeline = $null
            if ($rows -ne 48) {
                $pipeline = Read-Result "$engine-$model-$rows-r1/pipeline.json"
                Assert-Samples $pipeline.latency 120
            }
            $storage += [ordered]@{
                model = $model; engine = $engine; rows = $rows
                build_ms = (Summary $cases.build_ms)
                database_bytes = (Summary @($cases | ForEach-Object {
                    if ($engine -eq 'sqlite') { $_.database_bytes } else { $_.database_bytes_exact }
                }))
                peak_mib = (Summary @($cases | ForEach-Object {
                    if ($engine -eq 'lance') { Peak-MiB $_.memory_exact } else { Peak-MiB $_.memory }
                }))
                peak_including_ann_mib = (Summary @($cases | ForEach-Object { Peak-MiB $_.memory }))
                vector_p50_ms = (Summary $cases.vector.p50_ms); vector_p95_ms = (Summary $cases.vector.p95_ms)
                fts_p50_ms = (Summary $cases.fts.p50_ms); fts_p95_ms = (Summary $cases.fts.p95_ms)
                hybrid_p50_ms = (Summary $cases.hybrid_rrf.p50_ms); hybrid_p95_ms = (Summary $cases.hybrid_rrf.p95_ms)
                pipeline = $pipeline; quality = $cases[0].quality
                ann = @(foreach ($probes in @(1, 4, 16)) {
                    $annCases = @($cases.ann | Where-Object nprobes -eq $probes)
                    if ($annCases.Count) {
                        [ordered]@{ nprobes = $probes; partitions = $annCases[0].partitions;
                            p50_ms = (Summary $annCases.latency.p50_ms); p95_ms = (Summary $annCases.latency.p95_ms);
                            recall_at_10 = (Summary $annCases.recall_at_10_vs_exact);
                            build_ms = (Summary $annCases.build_ms);
                            database_bytes_with_ann = (Summary $cases.database_bytes_with_ann) }
                    }
                })
            }
        }
        foreach ($repeat in 1..3) {
            $sql = Read-Result "sqlite-$model-$rows-r$repeat/storage.json"
            $lance = Read-Result "lance-$model-$rows-r$repeat/storage.json"
            $overlap = @(for ($i = 0; $i -lt 24; $i++) {
                @($sql.first_vector_rankings[$i] | Where-Object { $lance.first_vector_rankings[$i] -contains $_ }).Count / 10
            })
            $agreement += [ordered]@{ model = $model; rows = $rows; repeat = $repeat;
                top10_set_overlap_mean = ($overlap | Measure-Object -Average).Average;
                top10_set_overlap_min = ($overlap | Measure-Object -Minimum).Minimum }
        }
    }
    foreach ($language in @('por_Latn', 'eng_Latn')) {
        $case = Read-Result "belebele-$model-$language/quality.json"
        if ($case.queries -ne 900 -or $case.documents -ne 488) { throw 'Wrong public dataset shape' }
        Assert-Quality $case.quality 900 488
        $public += [ordered]@{ model = $model; language = $language; result = $case }
    }
}
$result = [ordered]@{
    validation = 'PASS: 8 embedding cases, 48 storage cases, 12 measured pipelines, 4 public quality cases; sample counts, finite timings, unique bounded IDs, ANN recall, no model swap.'
    aggregation = 'Median/min/max of three process-level values, not pooled percentiles. Single-thread and pipeline have one process; length profiles have 20 samples/process.'
    embeddings = $embedding; storage = $storage; public_quality = $public; exact_engine_agreement = $agreement
}
$result | ConvertTo-Json -Depth 40 | Set-Content -LiteralPath (Join-Path $root 'summary.json') -Encoding utf8NoBOM
Write-Output $result.validation
Write-Output (Join-Path $root 'summary.json')
