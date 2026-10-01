mod corpus;
#[cfg(feature = "embedding")]
mod embedding;
#[cfg(feature = "lance")]
mod lance;
mod sqlite;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path, time::Instant};

#[derive(Serialize, Deserialize)]
pub struct Vectors {
    model: String,
    documents: Vec<Vec<f32>>,
    queries: Vec<Vec<f32>>,
}

pub fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

pub fn stats(samples: &[f64]) -> Value {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let percentile = |p: f64| sorted[(sorted.len() as f64 * p).ceil() as usize - 1];
    json!({"n": samples.len(), "p50_ms": percentile(0.5), "p95_ms": percentile(0.95),
        "p99_ms": percentile(0.99), "mean_ms": samples.iter().sum::<f64>() / samples.len() as f64,
        "samples_ms": samples})
}

pub fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

pub fn normalize(v: &mut [f32]) {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    for x in v {
        *x /= norm;
    }
}

// Scaling workload only: unique deterministic perturbations of real embeddings.
// These rows are NOT additional semantic examples and never enter quality scores.
pub fn scaled_vector(base: &[Vec<f32>], id: usize) -> Vec<f32> {
    if id < base.len() {
        return base[id].clone();
    }
    let mut state = id as u64 + 0x9e3779b97f4a7c15;
    let mut v = base[id % base.len()].clone();
    for x in &mut v {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *x += ((state >> 40) as f32 / 16777216.0 - 0.5) * 0.06;
    }
    normalize(&mut v);
    v
}

pub fn rrf(a: &[usize], b: &[usize], k: usize) -> Vec<usize> {
    let mut scores = BTreeMap::<usize, f64>::new();
    for list in [a, b] {
        for (rank, id) in list.iter().enumerate() {
            *scores.entry(*id).or_default() += 1.0 / (60.0 + rank as f64 + 1.0);
        }
    }
    let mut scores: Vec<_> = scores.into_iter().collect();
    scores.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    scores.into_iter().take(k).map(|(id, _)| id).collect()
}

pub fn quality(rankings: &[Vec<usize>]) -> Value {
    let (_, queries) = corpus::fixture();
    relevance_metrics(rankings, &queries, true)
}

pub fn relevance_metrics(
    rankings: &[Vec<usize>],
    queries: &[corpus::Query],
    fixture_negatives: bool,
) -> Value {
    let mut recall = 0.0;
    let mut mrr = 0.0;
    let mut ndcg = 0.0;
    let mut top1_relevant = 0;
    let mut top1_hard_negative = 0;
    for (q, hits) in queries.iter().zip(rankings) {
        if hits.first().is_some_and(|id| q.relevant.contains(id)) {
            top1_relevant += 1;
        }
        if fixture_negatives
            && hits
                .first()
                .is_some_and(|id| *id == q.relevant[0] + 2 || *id == q.relevant[0] + 3)
        {
            top1_hard_negative += 1;
        }
        recall += hits
            .iter()
            .take(10)
            .filter(|id| q.relevant.contains(id))
            .count() as f64
            / q.relevant.len() as f64;
        mrr += hits
            .iter()
            .take(10)
            .position(|id| q.relevant.contains(id))
            .map_or(0.0, |r| 1.0 / (r + 1) as f64);
        let dcg = hits
            .iter()
            .take(10)
            .enumerate()
            .filter(|(_, id)| q.relevant.contains(id))
            .map(|(r, _)| 1.0 / ((r + 2) as f64).log2())
            .sum::<f64>();
        let ideal = (0..q.relevant.len().min(10))
            .map(|r| 1.0 / ((r + 2) as f64).log2())
            .sum::<f64>();
        ndcg += dcg / ideal;
    }
    let n = queries.len() as f64;
    json!({"recall_at_10": recall/n, "mrr_at_10": mrr/n, "ndcg_at_10": ndcg/n,
        "top1_relevant_rate":top1_relevant as f64/n,
        "top1_hard_negative_rate":if fixture_negatives {
            Some(top1_hard_negative as f64/n)
        } else {None},
        "queries": queries.len(), "rankings": rankings})
}

#[cfg(feature = "embedding")]
pub fn exact(base: &[Vec<f32>], query: &[f32], k: usize) -> Vec<usize> {
    let mut scores: Vec<_> = base
        .iter()
        .enumerate()
        .map(|(id, v)| (id, v.iter().zip(query).map(|(a, b)| a * b).sum::<f32>()))
        .collect();
    scores.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    scores.into_iter().take(k).map(|x| x.0).collect()
}

pub fn memory() -> Value {
    // Linux kernel high-water mark includes model loading and subsequent work.
    // Runtime platform is recorded explicitly; this is not a Windows measurement.
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let mut fields: BTreeMap<_, _> = status
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            if ["VmHWM", "VmRSS", "VmPeak"].contains(&name) {
                Some((name.to_string(), value.trim().to_string()))
            } else {
                None
            }
        })
        .collect();
    for (name, file) in [
        ("cgroup_current_bytes", "/sys/fs/cgroup/memory.current"),
        ("cgroup_peak_bytes_so_far", "/sys/fs/cgroup/memory.peak"),
        (
            "cgroup_swap_current_bytes",
            "/sys/fs/cgroup/memory.swap.current",
        ),
    ] {
        if let Ok(value) = std::fs::read_to_string(file) {
            fields.insert(name.to_string(), value.trim().to_string());
        }
    }
    json!(fields)
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("fixture") => {
            let (docs, queries) = corpus::fixture();
            write_json(
                Path::new(&args[1]),
                &json!({"documents": docs, "queries": queries}),
            )?;
        }
        #[cfg(feature = "embedding")]
        Some("evaluate") => embedding::evaluate(&args[1..])?,
        #[cfg(feature = "embedding")]
        Some("pipeline") => match args[1].as_str() {
            "sqlite" => embedding::pipeline_sqlite(&args[2..])?,
            #[cfg(feature = "lance")]
            "lance" => {
                tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(4)
                    .enable_all()
                    .build()?
                    .block_on(lance::pipeline(&args[2..]))?;
            }
            _ => bail!("unsupported pipeline engine"),
        },
        #[cfg(feature = "embedding")]
        Some("embed" | "prepare") => {
            let mut options = args[1..].to_vec();
            if args[0] == "prepare" {
                options.push("prepare".into());
            }
            embedding::run(&options)?;
        }
        Some("sqlite") => sqlite::run(&args[1..])?,
        #[cfg(feature = "lance")]
        Some("lance") => {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .enable_all()
                .build()?
                .block_on(lance::run(&args[1..]))?;
        }
        _ => bail!("fixture OUT | embed MODEL THREADS CACHE OUT | sqlite/lance VECTORS ROWS OUT"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metrics_and_fusion_have_known_answers() {
        let (_, q) = corpus::fixture();
        let perfect: Vec<_> = q.iter().map(|x| x.relevant.clone()).collect();
        assert_eq!(quality(&perfect)["ndcg_at_10"], 1.0);
        assert_eq!(quality(&vec![vec![]; q.len()])["recall_at_10"], 0.0);
        assert_eq!(rrf(&[1, 2], &[2, 3], 3), vec![2, 1, 3]);
    }
    #[test]
    fn synthetic_vectors_are_reproducible_unique_and_normalized() {
        let base = vec![vec![1.0, 0.0, 0.0]];
        assert_eq!(scaled_vector(&base, 1), scaled_vector(&base, 1));
        assert_ne!(scaled_vector(&base, 1), scaled_vector(&base, 2));
        let norm = scaled_vector(&base, 1).iter().map(|x| x * x).sum::<f32>();
        assert!((norm - 1.0).abs() < 1e-6);
    }
}
