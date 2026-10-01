[CmdletBinding()]
param([Parameter(Mandatory)][string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$revision = '979a211276faa22f671e69d096634193567cfd05'
foreach ($language in @('por_Latn', 'eng_Latn')) {
    $raw = Join-Path $OutputDirectory "$language.jsonl"
    $url = "https://huggingface.co/datasets/mteb/belebele/resolve/$revision/data/$language.jsonl"
    if (-not (Test-Path -LiteralPath $raw)) { Invoke-WebRequest -Uri $url -OutFile $raw }
    $expected = if ($language -eq 'por_Latn') {
        'da63cd6215d550f0ad3df5562567096e8e7731a04322f90e801c554b1c43b431'
    } else { '15af884c2b5994fdc71199e79d553a1864b08ce1b31894aa87f6be925a064a31' }
    if ((Get-FileHash -LiteralPath $raw -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected) {
        throw "Dataset checksum mismatch: $language"
    }
    $links = [Collections.Generic.Dictionary[string,int]]::new([StringComparer]::Ordinal)
    $questions = [Collections.Generic.Dictionary[string,Collections.Generic.List[int]]]::new([StringComparer]::Ordinal)
    $questionOrder = [Collections.Generic.List[string]]::new()
    $documents = [Collections.Generic.List[object]]::new()
    foreach ($line in Get-Content -LiteralPath $raw -Encoding utf8) {
        $row = $line | ConvertFrom-Json
        if (-not $links.ContainsKey($row.link)) {
            $id = $documents.Count
            $links.Add($row.link, $id)
            $documents.Add([ordered]@{ id = $id; text = $row.flores_passage; source_link = $row.link })
        }
        if (-not $questions.ContainsKey($row.question)) {
            $questions.Add($row.question, [Collections.Generic.List[int]]::new())
            $questionOrder.Add($row.question)
        }
        $id = $links[$row.link]
        if (-not $questions[$row.question].Contains($id)) { $questions[$row.question].Add($id) }
    }
    $queries = @($questionOrder | ForEach-Object {
        [ordered]@{ text = $_; relevant = @($questions[$_]) }
    })
    $dataset = [ordered]@{
        source = $url
        source_sha256 = (Get-FileHash -LiteralPath $raw -Algorithm SHA256).Hash.ToLowerInvariant()
        source_revision = $revision
        attribution = 'Belebele, Meta/Facebook Research, 2023; MTEB retrieval conversion'
        license = 'CC-BY-SA-4.0'
        license_url = 'https://github.com/facebookresearch/belebele/blob/main/LICENSE_CC-BY-SA4.0'
        modifications = 'Passages deduplicated by link; exact duplicate questions merged; numeric IDs and binary qrels derived from original links; answer options omitted.'
        documents = @($documents)
        queries = $queries
    }
    $out = Join-Path $OutputDirectory "belebele-$language.json"
    $dataset | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $out -Encoding utf8NoBOM
    Write-Output "$language : $($documents.Count) passages, $($queries.Count) unique questions -> $out"
}
