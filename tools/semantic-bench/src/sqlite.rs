use crate::{corpus, elapsed_ms, memory, quality, rrf, scaled_vector, stats, write_json, Vectors};
use anyhow::{ensure, Result};
use rusqlite::{ffi::sqlite3_auto_extension, params, Connection};
use serde_json::json;
use std::{path::Path, time::Instant};

fn bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

pub(crate) fn vector(db: &Connection, q: &[f32], k: usize) -> Result<Vec<usize>> {
    Ok(db
        .prepare("SELECT rowid FROM vectors WHERE embedding MATCH ?1 AND k = ?2 ORDER BY distance")?
        .query_map(params![bytes(q), k as i64], |row| {
            row.get::<_, i64>(0).map(|id| id as usize)
        })?
        .collect::<rusqlite::Result<_>>()?)
}

pub fn lexical_query(text: &str) -> String {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| format!("\"{s}\""))
        .collect::<Vec<_>>()
        .join(" OR ")
}

pub(crate) fn lexical(db: &Connection, text: &str, k: usize) -> Result<Vec<usize>> {
    Ok(db
        .prepare(
            "SELECT rowid FROM texts WHERE texts MATCH ?1 ORDER BY bm25(texts), rowid LIMIT ?2",
        )?
        .query_map(params![lexical_query(text), k as i64], |r| {
            r.get::<_, i64>(0).map(|id| id as usize)
        })?
        .collect::<rusqlite::Result<_>>()?)
}

pub(crate) fn register() {
    type EntryPoint = unsafe extern "C" fn(
        *mut rusqlite::ffi::sqlite3,
        *mut *mut std::ffi::c_char,
        *const rusqlite::ffi::sqlite3_api_routines,
    ) -> std::ffi::c_int;
    // Documented static registration; SQLite ignores duplicate registrations.
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute::<*const (), EntryPoint>(
            sqlite_vec::sqlite3_vec_init as *const (),
        )));
    }
}

pub fn run(args: &[String]) -> Result<()> {
    let vectors: Vectors = serde_json::from_slice(&std::fs::read(&args[0])?)?;
    let count: usize = args[1].parse()?;
    let out = Path::new(&args[2]);
    std::fs::create_dir_all(out)?;
    let path = out.join("index.sqlite");
    ensure!(
        !path.exists(),
        "refusing to overwrite existing benchmark database"
    );
    register();
    let start = Instant::now();
    let mut db = Connection::open(&path)?;
    db.execute_batch(&format!(
        "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
        PRAGMA cache_size=-32768;
        CREATE VIRTUAL TABLE vectors USING vec0(embedding float[{}] distance_metric=cosine);
        CREATE VIRTUAL TABLE texts USING fts5(text, tokenize='unicode61 remove_diacritics 2');",
        vectors.documents[0].len()
    ))?;
    let (docs, queries) = corpus::fixture();
    let transaction = db.transaction()?;
    {
        let mut v_insert =
            transaction.prepare("INSERT INTO vectors(rowid,embedding) VALUES(?1,?2)")?;
        let mut t_insert = transaction.prepare("INSERT INTO texts(rowid,text) VALUES(?1,?2)")?;
        for id in 0..count {
            v_insert.execute(params![
                id as i64,
                bytes(&scaled_vector(&vectors.documents, id))
            ])?;
            t_insert.execute(params![id as i64, docs[id % docs.len()].text])?;
        }
    }
    transaction.commit()?;
    db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
    let build_ms = elapsed_ms(start);
    let version: String = db.query_row("SELECT vec_version()", [], |r| r.get(0))?;
    let sqlite_version: String = db.query_row("SELECT sqlite_version()", [], |r| r.get(0))?;
    for (i, q) in vectors.queries.iter().enumerate() {
        vector(&db, q, 10)?;
        lexical(&db, &queries[i].text, 10)?;
        rrf(
            &vector(&db, q, 20)?,
            &lexical(&db, &queries[i].text, 20)?,
            10,
        );
    }
    let mut vector_times = Vec::new();
    let mut fts_times = Vec::new();
    let mut hybrid_times = Vec::new();
    let mut vr = Vec::new();
    let mut fr = Vec::new();
    let mut hr = Vec::new();
    for repeat in 0..10 {
        for ordinal in 0..vectors.queries.len() {
            let i = (ordinal + repeat * 7) % vectors.queries.len();
            let q = &vectors.queries[i];
            let start = Instant::now();
            let hits = vector(&db, q, 10)?;
            vector_times.push(elapsed_ms(start));
            let start = Instant::now();
            let words = lexical(&db, &queries[i].text, 10)?;
            fts_times.push(elapsed_ms(start));
            let start = Instant::now();
            let merged = rrf(
                &vector(&db, q, 20)?,
                &lexical(&db, &queries[i].text, 20)?,
                10,
            );
            hybrid_times.push(elapsed_ms(start));
            if repeat == 0 {
                vr.push(hits);
                fr.push(words);
                hr.push(merged);
            }
        }
    }
    let fixture_quality = if count == docs.len() {
        json!({"vector":quality(&vr),"fts":quality(&fr),"hybrid":quality(&hr)})
    } else {
        json!(null)
    };
    let result = json!({"engine":"sqlite-vec+fts5", "vec_version":version,
        "sqlite_version":sqlite_version, "model":vectors.model,"rows":count,
        "dimension":vectors.documents[0].len(), "build_ms":build_ms,
        "database_bytes":std::fs::metadata(path)?.len(), "vector":stats(&vector_times),
        "fts":stats(&fts_times),"hybrid_rrf":stats(&hybrid_times),
        "quality":fixture_quality,
        "first_vector_rankings":vr,"memory":memory()});
    write_json(&out.join("storage.json"), &result)?;
    println!("{}", out.join("storage.json").display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extension_cosine_and_fts_punctuation() -> Result<()> {
        register();
        let db = Connection::open_in_memory()?;
        db.execute_batch(
            "CREATE VIRTUAL TABLE vectors USING vec0(embedding float[2] distance_metric=cosine);
            CREATE VIRTUAL TABLE texts USING fts5(text, tokenize='unicode61 remove_diacritics 2');",
        )?;
        for (id, v) in [[1.0, 0.0], [0.0, 1.0], [-1.0, 0.0]].iter().enumerate() {
            db.execute(
                "INSERT INTO vectors(rowid,embedding) VALUES(?1,?2)",
                params![id as i64, bytes(v)],
            )?;
        }
        db.execute(
            "INSERT INTO texts(rowid,text) VALUES(0,'decisão de busca')",
            [],
        )?;
        assert_eq!(vector(&db, &[1.0, 0.0], 2)?, vec![0, 1]);
        assert_eq!(lexical(&db, "decisao: busca?", 10)?, vec![0]);
        Ok(())
    }
}
