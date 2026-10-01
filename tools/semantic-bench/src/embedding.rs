use crate::{corpus, elapsed_ms, exact, memory, quality, stats, write_json, Vectors};
use anyhow::{bail, ensure, Result};
use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
use serde_json::json;
use std::{path::Path, time::Instant};
use tokenizers::Tokenizer;

fn model_id(name: &str) -> Result<EmbeddingModel> {
    match name {
        "e5" => Ok(EmbeddingModel::MultilingualE5Small),
        "gemma-q4" => Ok(EmbeddingModel::EmbeddingGemma300MQ4),
        _ => bail!("unsupported model"),
    }
}

pub(crate) fn initialize(name: &str, threads: usize, cache: &Path) -> Result<TextEmbedding> {
    Ok(TextEmbedding::try_new(
        TextInitOptions::new(model_id(name)?)
            .with_cache_dir(cache.to_path_buf())
            .with_intra_threads(threads)
            .with_max_length(512)
            .with_show_download_progress(false),
    )?)
}

pub(crate) fn query_text(name: &str, text: &str) -> String {
    if name == "e5" {
        format!("query: {text}")
    } else {
        format!("task: search result | query: {text}")
    }
}

fn document_text(name: &str, text: &str) -> String {
    if name == "e5" {
        format!("passage: {text}")
    } else {
        format!("title: none | text: {text}")
    }
}

struct Audit {
    documents: Vec<usize>,
    queries: Vec<usize>,
    profiles: Vec<(usize, usize, String)>,
}

// Audit BEFORE loading the model: two tokenizers must not be resident during
// inference. FastEmbed owns its own tokenizer privately.
fn audit(
    name: &str,
    cache: &Path,
    docs: &[String],
    queries: &[String],
    profiles: bool,
) -> Result<Audit> {
    let repo = TextEmbedding::get_model_info(&model_id(name)?)?
        .model_code
        .replace('/', "--");
    let directory = cache.join(format!("models--{repo}"));
    let revision = std::fs::read_to_string(directory.join("refs/main"))?;
    let file = directory
        .join("snapshots")
        .join(revision.trim())
        .join("tokenizer.json");
    let mut tokenizer = Tokenizer::from_file(file).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    tokenizer
        .with_truncation(None)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    tokenizer.with_padding(None);
    let tokens = |text: &str| -> Result<usize> {
        Ok(tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?
            .len())
    };
    let documents = docs.iter().map(|s| tokens(s)).collect::<Result<_>>()?;
    let queries = queries.iter().map(|s| tokens(s)).collect::<Result<_>>()?;
    let mut inputs = Vec::new();
    if profiles {
        for target in [128, 256, 512] {
            let phrase =
                "A decisão registra o componente de busca, a evidência da alteração e sua origem. ";
            let mut body = String::new();
            while tokens(&query_text(name, &(body.clone() + phrase)))? <= target {
                body.push_str(phrase);
            }
            let text = query_text(name, &body);
            inputs.push((target, tokens(&text)?, text));
        }
    }
    Ok(Audit {
        documents,
        queries,
        profiles: inputs,
    })
}

fn validate(documents: &[Vec<f32>], queries: &[Vec<f32>]) -> Result<()> {
    let dimension = documents[0].len();
    for v in documents.iter().chain(queries) {
        ensure!(
            v.len() == dimension && v.iter().all(|x| x.is_finite()),
            "invalid embedding"
        );
        ensure!(
            (v.iter().map(|x| x * x).sum::<f32>() - 1.0).abs() < 0.001,
            "embedding not normalized"
        );
    }
    Ok(())
}

pub fn evaluate(args: &[String]) -> Result<()> {
    let name = &args[0];
    let threads: usize = args[1].parse()?;
    let cache = Path::new(&args[2]);
    let data: corpus::Dataset = serde_json::from_slice(&std::fs::read(&args[3])?)?;
    let out = Path::new(&args[4]);
    let docs: Vec<_> = data
        .documents
        .iter()
        .map(|d| document_text(name, &d.text))
        .collect();
    let queries: Vec<_> = data
        .queries
        .iter()
        .map(|q| query_text(name, &q.text))
        .collect();
    let audit = audit(name, cache, &docs, &queries, false)?;
    let mut model = initialize(name, threads, cache)?;
    let start = Instant::now();
    let doc_vectors = model.embed(&docs, Some(8))?;
    let document_encoding_ms = elapsed_ms(start);
    let start = Instant::now();
    let query_vectors = model.embed(&queries, Some(8))?;
    let query_encoding_ms = elapsed_ms(start);
    validate(&doc_vectors, &query_vectors)?;
    let rankings: Vec<_> = query_vectors
        .iter()
        .map(|q| exact(&doc_vectors, q, 10))
        .collect();
    write_json(
        &out.join("quality.json"),
        &json!({
            "model": name, "dataset": args[3], "threads": threads, "max_tokens": 512,
            "documents": docs.len(), "queries": queries.len(),
            "document_encoding_ms": document_encoding_ms, "query_encoding_ms": query_encoding_ms,
            "truncated_documents": audit.documents.iter().filter(|n| **n>512).count(),
            "truncated_queries": audit.queries.iter().filter(|n| **n>512).count(),
            "document_tokens": audit.documents, "query_tokens": audit.queries,
            "quality": crate::relevance_metrics(&rankings, &data.queries, false), "memory": memory()
        }),
    )?;
    println!("{}", out.join("quality.json").display());
    Ok(())
}

pub fn run(args: &[String]) -> Result<()> {
    let name = &args[0];
    let threads: usize = args[1].parse()?;
    let cache = Path::new(&args[2]);
    let output = Path::new(&args[3]);
    if args.get(4).is_some_and(|s| s == "prepare") {
        let start = Instant::now();
        let _model = initialize(name, threads, cache)?;
        write_json(
            &output.join("preparation.json"),
            &json!({
                "download_and_load_ms": elapsed_ms(start), "memory": memory()
            }),
        )?;
        return Ok(());
    }
    let (docs, queries) = corpus::fixture();
    let passages: Vec<_> = docs.iter().map(|d| document_text(name, &d.text)).collect();
    let texts: Vec<_> = queries.iter().map(|q| query_text(name, &q.text)).collect();
    let audit = audit(name, cache, &passages, &texts, true)?;
    ensure!(
        audit
            .queries
            .iter()
            .chain(&audit.documents)
            .all(|n| *n <= 512),
        "fixture truncated"
    );
    let start = Instant::now();
    let mut model = initialize(name, threads, cache)?;
    let load_ms = elapsed_ms(start);
    let memory_after_initialization = memory();
    let start = Instant::now();
    let first = model.embed(&texts[..1], Some(1))?;
    let first_ms = elapsed_ms(start);
    for text in texts.iter().take(4) {
        model.embed([text], Some(1))?;
    }
    let mut times = Vec::new();
    let mut query_vectors = Vec::new();
    for repeat in 0..5 {
        for ordinal in 0..texts.len() {
            let text = &texts[(ordinal + repeat * 7) % texts.len()];
            let start = Instant::now();
            let result = model.embed([text], Some(1))?;
            times.push(elapsed_ms(start));
            if repeat == 0 {
                query_vectors.push(result[0].clone());
            }
        }
    }
    let memory_after_short_queries = memory();
    let mut batch_times = Vec::new();
    let mut document_vectors = Vec::new();
    for repeat in 0..3 {
        let start = Instant::now();
        let result = model.embed(&passages, Some(8))?;
        batch_times.push(elapsed_ms(start));
        if repeat == 0 {
            document_vectors = result;
        }
    }
    let mut length_profiles = Vec::new();
    let mut long_batch8 = Vec::new();
    for (target, actual_tokens, text) in audit.profiles {
        model.embed([&text], Some(1))?;
        let mut samples = Vec::new();
        for _ in 0..20 {
            let start = Instant::now();
            model.embed([&text], Some(1))?;
            samples.push(elapsed_ms(start));
        }
        length_profiles.push(json!({
            "target_tokens": target, "actual_tokens": actual_tokens, "latency": stats(&samples)
        }));
        if target == 512 {
            for _ in 0..3 {
                let start = Instant::now();
                model.embed(vec![&text; 8], Some(8))?;
                long_batch8.push(elapsed_ms(start));
            }
        }
    }
    let start = Instant::now();
    model.embed(&passages, Some(1))?;
    let corpus_batch_1_ms = elapsed_ms(start);
    validate(&document_vectors, &query_vectors)?;
    let rankings: Vec<_> = query_vectors
        .iter()
        .map(|q| exact(&document_vectors, q, 10))
        .collect();
    write_json(
        &output.join("vectors.json"),
        &Vectors {
            model: name.clone(),
            documents: document_vectors,
            queries: query_vectors,
        },
    )?;
    write_json(
        &output.join("embedding.json"),
        &json!({
            "model": name, "threads": threads, "max_tokens": 512, "dimension": first[0].len(),
            "initialization_ms": load_ms, "first_query_ms": first_ms, "warm_query": stats(&times),
            "batch_8_corpus": {"n": batch_times.len(), "samples_ms": batch_times,
                "mean_ms": batch_times.iter().sum::<f64>()/batch_times.len() as f64},
            "memory_after_initialization": memory_after_initialization,
            "memory_after_short_queries": memory_after_short_queries,
            "batch_1_corpus_ms": corpus_batch_1_ms, "length_profiles": length_profiles,
            "synthetic_long_batch8":{"n":long_batch8.len(),"samples_ms":long_batch8,
                "mean_ms":long_batch8.iter().sum::<f64>()/long_batch8.len() as f64},
            "query_tokens": audit.queries, "document_tokens": audit.documents,
            "documents": docs.len(), "quality": quality(&rankings),
            "provider": "CPU", "memory": memory()
        }),
    )?;
    println!("{}", output.join("embedding.json").display());
    Ok(())
}

pub fn pipeline_sqlite(args: &[String]) -> Result<()> {
    let name = &args[0];
    let threads = args[1].parse()?;
    let mut model = initialize(name, threads, Path::new(&args[2]))?;
    crate::sqlite::register();
    let db = rusqlite::Connection::open(&args[3])?;
    db.execute_batch("PRAGMA cache_size=-32768;")?;
    let (_, queries) = corpus::fixture();
    let mut samples = Vec::new();
    for repeat in 0..6 {
        for ordinal in 0..queries.len() {
            let i = (ordinal + repeat * 7) % queries.len();
            let start = Instant::now();
            let encoded = model.embed([query_text(name, &queries[i].text)], Some(1))?;
            let hits = crate::rrf(
                &crate::sqlite::vector(&db, &encoded[0], 20)?,
                &crate::sqlite::lexical(&db, &queries[i].text, 20)?,
                10,
            );
            let _ = std::hint::black_box(hits);
            if repeat > 0 {
                samples.push(elapsed_ms(start));
            }
        }
    }
    write_json(
        &Path::new(&args[4]).join("pipeline.json"),
        &json!({
            "model":name,"engine":"sqlite","threads":threads,
            "latency":stats(&samples),"memory":memory()
        }),
    )?;
    Ok(())
}
