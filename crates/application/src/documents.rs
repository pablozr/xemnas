//! Project documentation: the Markdown and text the project already keeps
//! (`docs/`, `specs/`, ADRs, README…), indexed locally so it counts as
//! knowledge next to decisions and rules.
//!
//! Documents are read, never trusted as decisions: they feed the Visão
//! (titles, sections and a short excerpt, cited like any record) and are
//! listed in Contexto. Indexing reads only the project folder, keeps a
//! fingerprint per file and never leaves the machine.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::captures::{
    CaptureArtifactRecord, CaptureCheckpointRecord, CaptureError, CaptureReceiptRecord,
    CaptureRepository, CaptureWrite,
};
use crate::jobs::{JobRecord, JobState, ANALYZE_DOCUMENT_KIND};
use crate::projects::ProjectRepository;
use crate::redact::redact_secrets;
use crate::repo_identity::{normalize_dir, RepoIdentity};

/// Folders (at the project root) whose files are documentation.
pub const DOC_DIRS: [&str; 10] = [
    "docs",
    "doc",
    "documentation",
    "specs",
    "spec",
    "adr",
    "adrs",
    "rfcs",
    "architecture",
    "design",
];

/// Files at the project root that are documentation.
pub const ROOT_DOCS: [&str; 7] = [
    "README.md",
    "ARCHITECTURE.md",
    "DESIGN.md",
    "CONTRIBUTING.md",
    "AGENTS.md",
    "CLAUDE.md",
    "SPEC.md",
];

/// Extensions read as documentation.
const EXTENSIONS: [&str; 6] = ["md", "mdx", "markdown", "txt", "rst", "adoc"];
/// Folders never entered.
const SKIPPED: [&str; 6] = ["node_modules", "target", "dist", "build", "vendor", "out"];
/// Most files indexed per project.
pub const MAX_DOCUMENTS: usize = 400;
/// Larger files are skipped (generated dumps, logs).
pub const MAX_FILE_BYTES: u64 = 512 * 1024;
/// Deepest folder level walked under a documentation folder.
const MAX_DEPTH: usize = 6;
/// Section headings kept per document.
const MAX_HEADINGS: usize = 12;
/// Characters of the opening paragraph kept.
const EXCERPT_CHARS: usize = 600;

mod digest;

pub use digest::{digest as digest_document, Digest, Verdict, DIGEST_CHARS};

/// Artifact kind of a document sent to extraction.
pub const DOCUMENT_ARTIFACT: &str = "document";
/// Adapter recorded on document captures.
pub const DOCUMENT_ADAPTER: &str = "xemnas-documents";
/// Runs a failed document analysis gets again on its own (an indexing run
/// that meets it); importing the file again retries it regardless, and so does
/// "Reprocessar" in Diagnostics.
pub const MAX_AUTOMATIC_ATTEMPTS: u32 = 3;
/// Documents queued for review per run, ADRs and specs first.
pub const PROPOSE_PER_RUN: usize = 12;
/// Characters of a document sent to extraction.
const DOCUMENT_CHARS: usize = 40_000;

/// What a document is, from its place and name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DocumentKind {
    /// An architecture decision record.
    Adr,
    /// A specification, RFC or design document.
    Spec,
    /// A README.
    Readme,
    /// Any other guide or note.
    Guide,
}

impl DocumentKind {
    /// Every kind, in display order.
    pub const ALL: [Self; 4] = [Self::Adr, Self::Spec, Self::Readme, Self::Guide];

    /// Stable literal.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Adr => "adr",
            Self::Spec => "spec",
            Self::Readme => "readme",
            Self::Guide => "guide",
        }
    }

    /// Parses a stored literal.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

/// One indexed document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDocument {
    /// Owning project.
    pub project_id: String,
    /// Path relative to the project root, with `/`.
    pub path: String,
    /// What it is.
    pub kind: DocumentKind,
    /// First heading, or the file name.
    pub title: String,
    /// Section headings, in order.
    pub headings: Vec<String>,
    /// The opening paragraph, as plain text.
    pub excerpt: String,
    /// Size on disk.
    pub bytes: u64,
    /// Content fingerprint (FNV-1a, hex).
    pub fingerprint: String,
    /// RFC 3339 time of the last indexing that saw it.
    pub indexed_at: String,
    /// Absolute file of a document imported from outside the project folder
    /// (a sibling worktree); `None` for documents found by indexing.
    pub source: Option<String>,
}

/// What importing one document did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedDocument {
    /// The document as stored.
    pub document: ProjectDocument,
    /// Whether this version was queued for extraction now. `false` when it
    /// was already queued or has nothing central to extract.
    pub queued: bool,
}

/// What an indexing found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentIndex {
    /// Documents now indexed.
    pub total: usize,
    /// New or changed since the previous indexing.
    pub changed: usize,
    /// Gone since the previous indexing.
    pub removed: usize,
    /// Whether the walk stopped at [`MAX_DOCUMENTS`].
    pub truncated: bool,
}

/// Failures, in product language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentError {
    /// The store failed; the detail goes to the log.
    Storage(String),
    /// The project does not exist.
    ProjectNotFound,
    /// The project folder cannot be read.
    FolderUnavailable,
    /// The file to import cannot be read.
    FileUnavailable,
    /// The file is not inside the project folder or a worktree of its repository.
    OutsideRepository,
    /// The file is not a documentation file (Markdown, text in a docs folder…).
    NotDocumentation,
    /// The file is binary, not UTF-8 or larger than [`MAX_FILE_BYTES`].
    NotText,
}

impl std::fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(_) => formatter.write_str("não foi possível ler a documentação"),
            Self::ProjectNotFound => formatter.write_str("projeto não encontrado"),
            Self::FolderUnavailable => formatter.write_str("a pasta do projeto não está acessível"),
            Self::FileUnavailable => formatter.write_str("não foi possível ler o arquivo"),
            Self::OutsideRepository => {
                formatter.write_str("o arquivo não é do projeto nem de um worktree dele")
            }
            Self::NotDocumentation => formatter.write_str("o arquivo não é um documento"),
            Self::NotText => formatter.write_str("o arquivo não é texto ou é grande demais"),
        }
    }
}

impl std::error::Error for DocumentError {}

/// Persistence of the index.
pub trait DocumentStore {
    /// The indexed documents of a project, by path.
    fn project_documents(&self, project_id: &str) -> Result<Vec<ProjectDocument>, DocumentError>;

    /// Replaces the index of a project.
    fn replace_documents(
        &self,
        project_id: &str,
        documents: &[ProjectDocument],
    ) -> Result<(), DocumentError>;

    /// Requeues the analysis job of the document version queued under
    /// `idempotency_key` when it failed, and only then (fewer than
    /// `max_attempts` runs so far, when given). Whether it was requeued.
    fn requeue_failed_analysis(
        &self,
        idempotency_key: &str,
        max_attempts: Option<u32>,
    ) -> Result<bool, DocumentError>;
}

/// The documentation use case.
#[derive(Clone)]
pub struct Documents<S> {
    store: S,
    map: Option<crate::graph::MapPreparer>,
}

impl<S: std::fmt::Debug> std::fmt::Debug for Documents<S> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Documents")
            .field("store", &self.store)
            .finish_non_exhaustive()
    }
}

impl<S> Documents<S>
where
    S: DocumentStore + ProjectRepository,
{
    /// Wraps the store.
    pub fn new(store: S) -> Self {
        Self { store, map: None }
    }

    /// Prepares the project's map (declared components and suggestions)
    /// whenever documents are indexed or queued, so the analysis they start
    /// already finds the components.
    #[must_use]
    pub fn with_map_preparer(mut self, preparer: crate::graph::MapPreparer) -> Self {
        self.map = Some(preparer);
        self
    }

    fn prepare_map(&self, project_id: &str) {
        if let Some(prepare) = &self.map {
            prepare(project_id);
        }
    }

    /// The indexed documents, by kind then path.
    ///
    /// # Errors
    ///
    /// `storage`.
    pub fn list(&self, project_id: &str) -> Result<Vec<ProjectDocument>, DocumentError> {
        let mut documents = self.store.project_documents(project_id)?;
        documents.sort_by(|left, right| (left.kind, &left.path).cmp(&(right.kind, &right.path)));
        Ok(documents)
    }

    /// Reads the project folder again and replaces the index.
    ///
    /// # Errors
    ///
    /// `project_not_found`, `folder_unavailable` or `storage`.
    pub fn index(&self, project_id: &str) -> Result<DocumentIndex, DocumentError> {
        let project = ProjectRepository::get(&self.store, project_id)
            .map_err(|error| DocumentError::Storage(error.to_string()))?
            .ok_or(DocumentError::ProjectNotFound)?;
        let root = PathBuf::from(&project.location);
        if !root.is_dir() {
            return Err(DocumentError::FolderUnavailable);
        }
        let previous = self.store.project_documents(project_id)?;
        let (found, truncated) = scan(&root);
        let now = crate::clock::now_rfc3339();
        let documents: Vec<ProjectDocument> = found
            .into_iter()
            .filter_map(|(relative, absolute)| read(project_id, &relative, &absolute, &now))
            .collect();
        // Imported documents live outside the folder and survive a re-read
        // for as long as their file does; a scanned path of the same name wins.
        let imported: Vec<ProjectDocument> = previous
            .iter()
            .filter(|old| !documents.iter().any(|document| document.path == old.path))
            .filter_map(|old| {
                let source = old.source.as_deref()?;
                let mut document = read(project_id, &old.path, Path::new(source), &now)?;
                document.source = old.source.clone();
                Some(document)
            })
            .collect();
        let mut documents = documents;
        documents.extend(imported);
        let changed = documents
            .iter()
            .filter(|document| {
                !previous
                    .iter()
                    .any(|old| old.path == document.path && old.fingerprint == document.fingerprint)
            })
            .count();
        let removed = previous
            .iter()
            .filter(|old| !documents.iter().any(|document| document.path == old.path))
            .count();
        self.store.replace_documents(project_id, &documents)?;
        self.prepare_map(project_id);
        Ok(DocumentIndex {
            total: documents.len(),
            changed,
            removed,
            truncated,
        })
    }
}

impl<S> Documents<S>
where
    S: DocumentStore + ProjectRepository + CaptureRepository,
{
    /// Queues new or changed documents for the same analysis as captured
    /// conversations: each version of a document becomes one capture with a
    /// `document` artifact and an analysis job, so its decisions and rules
    /// reach Revisão and, once confirmed, the map's suggestions. A version
    /// already queued is not queued again, unless its analysis failed: then it
    /// is requeued, up to [`MAX_AUTOMATIC_ATTEMPTS`] runs in all. Returns how
    /// many were queued.
    ///
    /// # Errors
    ///
    /// `project_not_found` or `storage`.
    pub fn propose(&self, project_id: &str, limit: usize) -> Result<usize, DocumentError> {
        self.propose_matching(project_id, limit, None)
    }

    /// Imports one documentation file from the project folder or from a git
    /// worktree of the same repository, then queues it like any document.
    ///
    /// The stored path is relative to the file's own worktree root, so a
    /// document of a sibling worktree gets the path it will have once merged.
    /// Importing the same content again changes nothing; changed content
    /// replaces the stored version and queues the new one. A version whose
    /// analysis failed is requeued, so a fix takes effect without editing it.
    ///
    /// # Errors
    ///
    /// `project_not_found`, `folder_unavailable`, `file_unavailable`,
    /// `outside_repository`, `not_documentation`, `not_text` or `storage`.
    /// Nothing is stored when it fails.
    pub fn import_file(
        &self,
        identity: &impl RepoIdentity,
        project_id: &str,
        file: &Path,
    ) -> Result<ImportedDocument, DocumentError> {
        let project = ProjectRepository::get(&self.store, project_id)
            .map_err(|error| DocumentError::Storage(error.to_string()))?
            .ok_or(DocumentError::ProjectNotFound)?;
        let root = normalize_dir(&project.location).ok_or(DocumentError::FolderUnavailable)?;
        let file = normalize_dir(&file.to_string_lossy())
            .map(PathBuf::from)
            .filter(|file| file.is_file())
            .ok_or(DocumentError::FileUnavailable)?;
        let relative = repository_path(identity, Path::new(&root), &file)?;
        if !is_documentation_path(&relative) {
            return Err(DocumentError::NotDocumentation);
        }
        let metadata = std::fs::metadata(&file).map_err(|_| DocumentError::FileUnavailable)?;
        if metadata.len() > MAX_FILE_BYTES {
            return Err(DocumentError::NotText);
        }
        let raw = std::fs::read(&file).map_err(|_| DocumentError::FileUnavailable)?;
        if raw.contains(&0) || std::str::from_utf8(&raw).is_err() {
            return Err(DocumentError::NotText);
        }

        let mut document = build(project_id, &relative, &raw, &crate::clock::now_rfc3339());
        document.source = Some(file.to_string_lossy().into_owned());
        let mut documents = self.store.project_documents(project_id)?;
        let unchanged = documents
            .iter()
            .any(|old| old.path == relative && old.fingerprint == document.fingerprint);
        if !unchanged {
            documents.retain(|old| old.path != relative);
            documents.push(document.clone());
            self.store.replace_documents(project_id, &documents)?;
        }
        let queued = self.propose_matching(project_id, 1, Some(&relative))? > 0;
        Ok(ImportedDocument { document, queued })
    }

    fn propose_matching(
        &self,
        project_id: &str,
        limit: usize,
        only: Option<&str>,
    ) -> Result<usize, DocumentError> {
        let project = ProjectRepository::get(&self.store, project_id)
            .map_err(|error| DocumentError::Storage(error.to_string()))?
            .ok_or(DocumentError::ProjectNotFound)?;
        self.prepare_map(project_id);
        let root = PathBuf::from(&project.location);
        // What would be queued: new or changed documents that have central
        // parts, with only those parts. The rest never reaches Revisão.
        let mut candidates: Vec<(ProjectDocument, digest::Digest)> = Vec::new();
        let mut requeued = 0;
        for document in self.list(project_id)? {
            if only.is_some_and(|path| path != document.path) {
                continue;
            }
            let key = document_capture_key(project_id, &document);
            if self
                .store
                .find_receipt_by_idempotency_key(&key)
                .map_err(capture_error)?
                .is_some()
            {
                // Queued before. Only a failed analysis runs again: always
                // when the person asked for this very file, else a few times.
                let cap = only.is_none().then_some(MAX_AUTOMATIC_ATTEMPTS);
                if requeued < limit && self.store.requeue_failed_analysis(&key, cap)? {
                    requeued += 1;
                }
                continue;
            }
            let file = document
                .source
                .as_deref()
                .map_or_else(|| root.join(&document.path), PathBuf::from);
            let Ok(raw) = std::fs::read(file) else {
                continue;
            };
            let content: String = redact_secrets(&String::from_utf8_lossy(&raw))
                .chars()
                .take(DOCUMENT_CHARS)
                .collect();
            if content.trim().is_empty() {
                continue;
            }
            let digest = digest::digest(document.kind, &document.path, &content);
            if digest.verdict == digest::Verdict::Central {
                candidates.push((document, digest));
            }
        }
        // ADRs and specifications first, and within a kind the ones that
        // speak most in decisions.
        candidates.sort_by(|left, right| {
            (left.0.kind, std::cmp::Reverse(left.1.score), &left.0.path).cmp(&(
                right.0.kind,
                std::cmp::Reverse(right.1.score),
                &right.0.path,
            ))
        });
        let mut queued = requeued;
        for (document, digest) in candidates {
            if queued >= limit {
                break;
            }
            let key = document_capture_key(project_id, &document);
            match self.store.insert_capture(&document_capture(
                &project.location,
                &key,
                &document,
                digest,
            )) {
                Ok(()) => queued += 1,
                Err(CaptureError::DuplicateIdempotencyKey) => {}
                Err(error) => return Err(capture_error(error)),
            }
        }
        Ok(queued)
    }
}

/// One version of a document: project, path and content fingerprint.
fn document_capture_key(project_id: &str, document: &ProjectDocument) -> String {
    format!(
        "document:{project_id}:{}:{}",
        document.path, document.fingerprint
    )
}

fn document_capture(
    location: &str,
    key: &str,
    document: &ProjectDocument,
    digest: digest::Digest,
) -> CaptureWrite {
    let content = digest.text;
    let now = crate::clock::now_rfc3339();
    let capture_id = uuid::Uuid::now_v7().to_string();
    let metadata = serde_json::json!({
        "file": document.path,
        "doc_kind": document.kind.as_str(),
        "title": document.title,
        "sections": digest.sections,
    })
    .to_string();
    CaptureWrite {
        receipt: CaptureReceiptRecord {
            capture_id: capture_id.clone(),
            idempotency_key: key.to_string(),
            canonical_path: location.to_string(),
            received_at: now.clone(),
            artifact_count: 1,
        },
        artifacts: vec![CaptureArtifactRecord {
            capture_id: capture_id.clone(),
            artifact_id: uuid::Uuid::now_v7().to_string(),
            kind: DOCUMENT_ARTIFACT.to_string(),
            fingerprint: integration_contracts::capture::artifact_fingerprint(&content),
            content,
            metadata,
        }],
        job: JobRecord {
            id: uuid::Uuid::now_v7().to_string(),
            kind: ANALYZE_DOCUMENT_KIND.to_string(),
            payload: capture_id.clone(),
            state: JobState::Queued,
            idempotent: true,
            attempts: 0,
            last_error: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        checkpoint: CaptureCheckpointRecord {
            adapter: DOCUMENT_ADAPTER.to_string(),
            adapter_version: "1".to_string(),
            session_id: format!("documents:{}", document.kind.as_str()),
            message_id: format!("{}@{}", document.path, document.fingerprint),
            capture_id,
            observed_at: now.clone(),
            updated_at: now,
        },
    }
}

fn capture_error(error: CaptureError) -> DocumentError {
    DocumentError::Storage(error.to_string())
}

/// Documentation files under `root`: root files first, then documentation
/// folders, in a stable order.
pub fn scan(root: &Path) -> (Vec<(String, PathBuf)>, bool) {
    let mut found = Vec::new();
    for name in ROOT_DOCS {
        let path = root.join(name);
        if path.is_file() {
            found.push((name.to_string(), path));
        }
    }
    let mut truncated = false;
    for folder in DOC_DIRS {
        let path = root.join(folder);
        if path.is_dir() {
            walk(&path, folder, 0, &mut found, &mut truncated);
        }
    }
    found.dedup_by(|left, right| left.0 == right.0);
    if found.len() > MAX_DOCUMENTS {
        found.truncate(MAX_DOCUMENTS);
        truncated = true;
    }
    (found, truncated)
}

fn walk(
    folder: &Path,
    relative: &str,
    depth: usize,
    found: &mut Vec<(String, PathBuf)>,
    truncated: &mut bool,
) {
    if depth > MAX_DEPTH || found.len() >= MAX_DOCUMENTS {
        *truncated |= found.len() >= MAX_DOCUMENTS;
        return;
    }
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let child = format!("{relative}/{name}");
        if kind.is_dir() {
            if !SKIPPED.contains(&name.as_str()) {
                walk(&entry.path(), &child, depth + 1, found, truncated);
            }
        } else if kind.is_file() && documentation_extension(&name) {
            if found.len() >= MAX_DOCUMENTS {
                *truncated = true;
                return;
            }
            found.push((child, entry.path()));
        }
    }
}

fn documentation_extension(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, extension)| EXTENSIONS.contains(&extension.to_lowercase().as_str()))
}

/// Whether a project-relative path is documentation: a Markdown, reST or
/// AsciiDoc file anywhere, or a `.txt` inside a documentation folder
/// (`requirements.txt` at the root is a manifest, not a document).
///
/// Documentation is evidence of a decision, not the part it affects: the map
/// does not derive `affects` from these paths (ADR-0005).
pub fn is_documentation_path(path: &str) -> bool {
    let path = domain::entities::normalize_path(path).to_lowercase();
    let name = path.rsplit('/').next().unwrap_or(&path);
    if !documentation_extension(name) {
        return false;
    }
    if !name.ends_with(".txt") {
        return true;
    }
    path.split('/')
        .next()
        .is_some_and(|folder| folder != name && DOC_DIRS.contains(&folder))
}

/// Path of `file` relative to the root of the worktree it lives in, with `/`:
/// the project folder itself, or any worktree of the same repository.
fn repository_path(
    identity: &impl RepoIdentity,
    root: &Path,
    file: &Path,
) -> Result<String, DocumentError> {
    let slashes = |path: &Path| path.to_string_lossy().replace('\\', "/");
    if let Ok(relative) = file.strip_prefix(root) {
        return Ok(slashes(relative));
    }
    let directory = file.parent().ok_or(DocumentError::OutsideRepository)?;
    let directory = directory.to_string_lossy();
    let project_repository = identity.common_dir(&root.to_string_lossy());
    let file_repository = identity.common_dir(&directory);
    if project_repository.is_none() || project_repository != file_repository {
        return Err(DocumentError::OutsideRepository);
    }
    let top = identity
        .worktree_root(&directory)
        .ok_or(DocumentError::OutsideRepository)?;
    file.strip_prefix(&top)
        .map(slashes)
        .map_err(|_| DocumentError::OutsideRepository)
}

fn read(project_id: &str, relative: &str, path: &Path, now: &str) -> Option<ProjectDocument> {
    if std::fs::metadata(path).ok()?.len() > MAX_FILE_BYTES {
        return None;
    }
    let raw = std::fs::read(path).ok()?;
    Some(build(project_id, relative, &raw, now))
}

fn build(project_id: &str, relative: &str, raw: &[u8], now: &str) -> ProjectDocument {
    let content = String::from_utf8_lossy(raw);
    let outline = outline(&content);
    let file_name = relative.rsplit('/').next().unwrap_or(relative);
    ProjectDocument {
        project_id: project_id.to_string(),
        path: relative.to_string(),
        kind: classify(relative),
        title: outline.title.unwrap_or_else(|| {
            file_name
                .rsplit_once('.')
                .map_or(file_name, |(stem, _)| stem)
                .to_string()
        }),
        headings: outline.headings,
        excerpt: outline.excerpt,
        bytes: raw.len() as u64,
        fingerprint: fingerprint(raw),
        indexed_at: now.to_string(),
        source: None,
    }
}

/// What a document is, from its path.
pub fn classify(path: &str) -> DocumentKind {
    let lower = path.to_lowercase();
    let file = lower.rsplit('/').next().unwrap_or(&lower);
    let numbered = file
        .split(['-', '_'])
        .next()
        .is_some_and(|prefix| prefix.len() >= 3 && prefix.chars().all(|c| c.is_ascii_digit()));
    if lower
        .split('/')
        .any(|part| part == "adr" || part == "adrs" || part == "decisions")
        || file.starts_with("adr")
        || (numbered && lower.contains("adr"))
    {
        DocumentKind::Adr
    } else if file.starts_with("readme") {
        DocumentKind::Readme
    } else if lower.split('/').any(|part| {
        matches!(
            part,
            "spec" | "specs" | "rfcs" | "rfc" | "design" | "architecture"
        )
    }) || file.contains("spec")
        || file.starts_with("rfc")
    {
        DocumentKind::Spec
    } else {
        DocumentKind::Guide
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Outline {
    title: Option<String>,
    headings: Vec<String>,
    excerpt: String,
}

/// Title, section headings and opening paragraph of a Markdown text.
fn outline(content: &str) -> Outline {
    let mut outline = Outline::default();
    let mut paragraph: Vec<String> = Vec::new();
    let mut fenced = false;
    let mut front_matter = content.trim_start().starts_with("---");
    let mut seen_front = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if front_matter {
            if trimmed == "---" {
                if seen_front {
                    front_matter = false;
                }
                seen_front = true;
            }
            continue;
        }
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        if let Some(heading) = heading(trimmed) {
            if heading.0 == 1 && outline.title.is_none() {
                outline.title = Some(heading.1);
            } else if outline.headings.len() < MAX_HEADINGS {
                outline.headings.push(heading.1);
            }
            if !paragraph.is_empty() && outline.excerpt.is_empty() {
                outline.excerpt = plain(&paragraph.join(" "));
            }
            paragraph.clear();
            continue;
        }
        if trimmed.is_empty() {
            if !paragraph.is_empty() && outline.excerpt.is_empty() {
                outline.excerpt = plain(&paragraph.join(" "));
            }
            paragraph.clear();
            continue;
        }
        if outline.excerpt.is_empty()
            && !trimmed.starts_with('|')
            && !trimmed.starts_with("<!--")
            && !trimmed.starts_with("![")
        {
            paragraph.push(trimmed.trim_start_matches(['-', '*', '>', ' ']).to_string());
        }
    }
    if outline.excerpt.is_empty() && !paragraph.is_empty() {
        outline.excerpt = plain(&paragraph.join(" "));
    }
    outline.excerpt = outline.excerpt.chars().take(EXCERPT_CHARS).collect();
    outline
}

fn heading(line: &str) -> Option<(usize, String)> {
    let level = line.chars().take_while(|c| *c == '#').count();
    if !(1..=4).contains(&level) {
        return None;
    }
    let text = line[level..].trim();
    (!text.is_empty() && line[level..].starts_with(' ')).then(|| (level, plain(text)))
}

/// Markdown emphasis, code and links reduced to their text.
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' | '_' | '`' => {}
            '[' => {
                let mut label = String::new();
                for next in chars.by_ref() {
                    if next == ']' {
                        break;
                    }
                    label.push(next);
                }
                if chars.peek() == Some(&'(') {
                    for next in chars.by_ref() {
                        if next == ')' {
                            break;
                        }
                    }
                }
                out.push_str(&label);
            }
            _ => out.push(c),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Short, stable reference of a document for citations (`F:` + 8 hex of
/// its path): paths share endings (`README.md`), so the tail would collide.
pub fn document_ref(path: &str) -> String {
    fingerprint(path.as_bytes())[..8].to_string()
}

/// FNV-1a 64 of the bytes, hex.
fn fingerprint(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documentation_paths_are_text_documents_not_manifests() {
        assert!(is_documentation_path("docs/arquitetura/adr/0005-grafo.md"));
        assert!(is_documentation_path("README.md"));
        assert!(is_documentation_path("crates\\core\\NOTES.MD"));
        assert!(is_documentation_path("docs/notas.txt"));
        assert!(!is_documentation_path("requirements.txt"));
        assert!(!is_documentation_path("crates/core/src/lib.rs"));
        assert!(!is_documentation_path("docs/build.rs"));
    }

    #[test]
    fn kinds_come_from_place_and_name() {
        assert_eq!(
            classify("docs/arquitetura/adr/0005-grafo.md"),
            DocumentKind::Adr
        );
        assert_eq!(classify("adr/0001-use-sqlite.md"), DocumentKind::Adr);
        assert_eq!(classify("README.md"), DocumentKind::Readme);
        assert_eq!(classify("docs/README.md"), DocumentKind::Readme);
        assert_eq!(classify("specs/capture.md"), DocumentKind::Spec);
        assert_eq!(classify("docs/api-spec.md"), DocumentKind::Spec);
        assert_eq!(classify("docs/guia.md"), DocumentKind::Guide);
    }

    #[test]
    fn the_outline_skips_front_matter_and_code() {
        let text = "---\ntitle: x\n---\n# Captura\n\nO **app** guarda [decisões](a.md) \
                    localmente.\nSem `rede`.\n\n```rust\n# not a heading\n```\n## Fluxo\n\
                    texto\n### Detalhe\n";
        let outline = outline(text);
        assert_eq!(outline.title.as_deref(), Some("Captura"));
        assert_eq!(outline.headings, vec!["Fluxo", "Detalhe"]);
        assert_eq!(
            outline.excerpt,
            "O app guarda decisões localmente. Sem rede."
        );
    }

    #[test]
    fn scanning_reads_only_documentation_folders() {
        let root = std::env::temp_dir().join(format!("xemnas-docs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("docs/adr")).unwrap();
        std::fs::create_dir_all(root.join("docs/node_modules")).unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("README.md"), "# App\n").unwrap();
        std::fs::write(root.join("docs/adr/0001-db.md"), "# Banco\n").unwrap();
        std::fs::write(root.join("docs/node_modules/x.md"), "x").unwrap();
        std::fs::write(root.join("docs/image.png"), "x").unwrap();
        std::fs::write(root.join("src/notes.md"), "x").unwrap();
        let (found, truncated) = scan(&root);
        let paths: Vec<&str> = found.iter().map(|(path, _)| path.as_str()).collect();
        assert_eq!(paths, vec!["README.md", "docs/adr/0001-db.md"]);
        assert!(!truncated);
        let _ = std::fs::remove_dir_all(&root);
    }
}
