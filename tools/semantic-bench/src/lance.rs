use crate::{corpus, elapsed_ms, memory, quality, rrf, scaled_vector, stats, write_json, Vectors};
use anyhow::{ensure, Result};
use arrow_array::{
    types::Float32Type, FixedSizeListArray, Int32Array, RecordBatch, RecordBatchIterator,
    StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::{
    index::{
        scalar::{FtsIndexBuilder, FullTextSearchQuery},
        vector::IvfFlatIndexBuilder,
        Index,
    },
    query::{ExecutableQuery, QueryBase, Select},
    DistanceType, Table,
};
use serde_json::json;
use std::{path::Path, sync::Arc, time::Instant};

async fn ids(stream: lancedb::arrow::SendableRecordBatchStream) -> Result<Vec<usize>> {
    let batches: Vec<RecordBatch> = stream.try_collect().await?;
    let mut ids = Vec::new();
    for batch in batches {
        let col = batch
            .column_by_name("id")
            .unwrap()
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap();
        ids.extend(col.values().iter().map(|x| *x as usize));
    }
    Ok(ids)
}

async fn vector(table: &Table, q: &[f32], k: usize, probes: Option<usize>) -> Result<Vec<usize>> {
    let query = table
        .query()
        .nearest_to(q)?
        .distance_type(DistanceType::Cosine)
        .select(Select::Columns(vec!["id".into()]))
        .limit(k);
    let query = if let Some(n) = probes {
        query.nprobes(n)
    } else {
        query.bypass_vector_index()
    };
    ids(query.execute().await?).await
}

async fn lexical(table: &Table, text: &str, k: usize) -> Result<Vec<usize>> {
    let terms = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    ids(table
        .query()
        .full_text_search(FullTextSearchQuery::new(terms))
        .select(Select::Columns(vec!["id".into()]))
        .limit(k)
        .execute()
        .await?)
    .await
}

fn tree_bytes(path: &Path) -> Result<u64> {
    let mut total = 0;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        total += if entry.file_type()?.is_dir() {
            tree_bytes(&entry.path())?
        } else {
            entry.metadata()?.len()
        };
    }
    Ok(total)
}

pub async fn run(args: &[String]) -> Result<()> {
    let vectors: Vectors = serde_json::from_slice(&std::fs::read(&args[0])?)?;
    let count: usize = args[1].parse()?;
    let out = Path::new(&args[2]);
    let path = out.join("lancedb");
    ensure!(
        !path.exists(),
        "refusing to overwrite existing benchmark database"
    );
    std::fs::create_dir_all(out)?;
    let dim = vectors.documents[0].len();
    let (docs, queries) = corpus::fixture();
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("text", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                dim as i32,
            ),
            true,
        ),
    ]));
    // Build in 1024-row batches: never allocate the entire scaled corpus in RAM.
    let data_docs: Vec<_> = docs.iter().map(|d| d.text.clone()).collect();
    let data_vectors = vectors.documents.clone();
    let batch_schema = schema.clone();
    let batches = (0..count).step_by(1024).map(move |start| {
        let end = (start + 1024).min(count);
        RecordBatch::try_new(
            batch_schema.clone(),
            vec![
                Arc::new(Int32Array::from_iter_values(
                    (start..end).map(|id| id as i32),
                )),
                Arc::new(StringArray::from_iter_values(
                    (start..end).map(|id| &data_docs[id % data_docs.len()]),
                )),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        (start..end).map(|id| {
                            Some(
                                scaled_vector(&data_vectors, id)
                                    .into_iter()
                                    .map(Some)
                                    .collect::<Vec<_>>(),
                            )
                        }),
                        dim as i32,
                    ),
                ),
            ],
        )
    });
    let start = Instant::now();
    let db = lancedb::connect(path.to_str().unwrap()).execute().await?;
    let reader: Box<dyn arrow_array::RecordBatchReader + Send> =
        Box::new(RecordBatchIterator::new(batches, schema));
    let table = db.create_table("decisions", reader).execute().await?;
    let insert_ms = elapsed_ms(start);
    let start = Instant::now();
    table
        .create_index(
            &["text"],
            Index::FTS(
                FtsIndexBuilder::default()
                    .stem(false)
                    .remove_stop_words(false)
                    .lower_case(true)
                    .ascii_folding(true),
            ),
        )
        .execute()
        .await?;
    let fts_build_ms = elapsed_ms(start);
    for (i, q) in vectors.queries.iter().enumerate() {
        vector(&table, q, 10, None).await?;
        lexical(&table, &queries[i].text, 10).await?;
        rrf(
            &vector(&table, q, 20, None).await?,
            &lexical(&table, &queries[i].text, 20).await?,
            10,
        );
    }
    let mut vt = Vec::new();
    let mut ft = Vec::new();
    let mut ht = Vec::new();
    let mut vr = Vec::new();
    let mut fr = Vec::new();
    let mut hr = Vec::new();
    for repeat in 0..10 {
        for ordinal in 0..vectors.queries.len() {
            let i = (ordinal + repeat * 7) % vectors.queries.len();
            let q = &vectors.queries[i];
            let start = Instant::now();
            let hits = vector(&table, q, 10, None).await?;
            vt.push(elapsed_ms(start));
            let start = Instant::now();
            let words = lexical(&table, &queries[i].text, 10).await?;
            ft.push(elapsed_ms(start));
            let start = Instant::now();
            let merged = rrf(
                &vector(&table, q, 20, None).await?,
                &lexical(&table, &queries[i].text, 20).await?,
                10,
            );
            ht.push(elapsed_ms(start));
            if repeat == 0 {
                vr.push(hits);
                fr.push(words);
                hr.push(merged);
            }
        }
    }
    let exact_bytes = tree_bytes(&path)?;
    let memory_exact = memory();
    let mut ann = Vec::new();
    if count >= 1000 {
        let partitions = (count as f64).sqrt() as u32;
        let start = Instant::now();
        table
            .create_index(
                &["vector"],
                Index::IvfFlat(
                    IvfFlatIndexBuilder::default()
                        .distance_type(DistanceType::Cosine)
                        .num_partitions(partitions),
                ),
            )
            .execute()
            .await?;
        let ann_build_ms = elapsed_ms(start);
        for probes in [1, 4, 16] {
            for q in &vectors.queries {
                vector(&table, q, 10, Some(probes)).await?;
            }
            let mut times = Vec::new();
            let mut recall = 0.0;
            for repeat in 0..10 {
                for ordinal in 0..vectors.queries.len() {
                    let i = (ordinal + repeat * 7) % vectors.queries.len();
                    let q = &vectors.queries[i];
                    let start = Instant::now();
                    let hits = vector(&table, q, 10, Some(probes)).await?;
                    times.push(elapsed_ms(start));
                    if repeat == 0 {
                        recall += hits.iter().filter(|id| vr[i].contains(id)).count() as f64 / 10.0;
                    }
                }
            }
            ann.push(
                json!({"index":"IVF_FLAT", "partitions":partitions,"nprobes":probes,
                "build_ms":ann_build_ms,"recall_at_10_vs_exact":recall/vectors.queries.len() as f64,
                "latency":stats(&times)}),
            );
        }
    }
    let fixture_quality = if count == docs.len() {
        json!({"vector":quality(&vr),"fts":quality(&fr),"hybrid":quality(&hr)})
    } else {
        json!(null)
    };
    write_json(
        &out.join("storage.json"),
        &json!({"engine":"lancedb-0.39.0", "model":vectors.model,
        "rows":count,"dimension":dim,"insert_ms":insert_ms,"fts_build_ms":fts_build_ms,
        "build_ms":insert_ms+fts_build_ms,"database_bytes_exact":exact_bytes,
        "database_bytes_with_ann":tree_bytes(&path)?,"vector":stats(&vt),"fts":stats(&ft),
        "hybrid_rrf":stats(&ht),"ann":ann,"first_vector_rankings":vr,
        "memory_exact":memory_exact,"memory":memory(),
        "quality":fixture_quality}),
    )?;
    println!("{}", out.join("storage.json").display());
    Ok(())
}

#[cfg(feature = "embedding")]
pub async fn pipeline(args: &[String]) -> Result<()> {
    let name = &args[0];
    let threads = args[1].parse()?;
    let mut model = crate::embedding::initialize(name, threads, Path::new(&args[2]))?;
    let db = lancedb::connect(&args[3]).execute().await?;
    let table = db.open_table("decisions").execute().await?;
    let (_, queries) = corpus::fixture();
    let mut samples = Vec::new();
    for repeat in 0..6 {
        for ordinal in 0..queries.len() {
            let i = (ordinal + repeat * 7) % queries.len();
            let start = Instant::now();
            let encoded = model.embed(
                [crate::embedding::query_text(name, &queries[i].text)],
                Some(1),
            )?;
            let hits = rrf(
                &vector(&table, &encoded[0], 20, None).await?,
                &lexical(&table, &queries[i].text, 20).await?,
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
            "model":name,"engine":"lance","threads":threads,
            "latency":stats(&samples),"memory":memory()
        }),
    )?;
    Ok(())
}
