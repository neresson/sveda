use std::fs;
use std::path::{Path, PathBuf};

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "vendor",
    ".git",
    "dist",
    "build",
    ".next",
    "coverage",
    "bootstrap/cache",
    ".idea",
    ".vscode",
    "__pycache__",
    "venv",
];

const EXTENSIONS: &[&str] = &[
    "php",
    "vue",
    "js",
    "jsx",
    "mjs",
    "cjs",
    "ts",
    "tsx",
    "css",
    "scss",
    "sass",
    "less",
    "md",
    "mdx",
    "json",
    "yaml",
    "yml",
    "xml",
    "html",
    "blade.php",
    "sql",
    "sh",
    "env.example",
    "ini",
    "toml",
    "rs",
    "go",
    "java",
    "kt",
    "swift",
];

const MAX_FILE_BYTES: usize = 512 * 1024;
const DEFAULT_CHUNK_CHARS: usize = 1600;
pub const EMBED_DIMS: usize = 256;
const MAX_CHUNKS: usize = 8000;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Chunk {
    pub start_line: usize,
    pub end_line: usize,
    pub text: String,
}

#[derive(Clone, Debug, Default)]
pub struct GrepHit {
    pub path: String,
    pub line: usize,
    pub text: String,
}

#[derive(Clone, Debug, Default)]
pub struct SearchHit {
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub text: String,
    pub score: f32,
}

#[derive(Clone, Debug, Default)]
pub struct FileSlice {
    pub path: String,
    pub content: String,
    pub offset: usize,
    pub length: usize,
    pub total: usize,
    pub has_more: bool,
}

#[derive(Clone, Debug)]
struct IndexedFile {
    path: String,
    content: String,
}

#[derive(Clone, Debug)]
struct IndexedChunk {
    path: String,
    start_line: usize,
    end_line: usize,
    text: String,
    embedding: Vec<f32>,
}

#[derive(Clone, Debug)]
struct ChunkDraft {
    path: String,
    start_line: usize,
    end_line: usize,
    text: String,
    embed_source: String,
}

#[derive(Clone, Debug)]
pub struct CollectedWorkspace {
    pub root: PathBuf,
    files: Vec<IndexedFile>,
    drafts: Vec<ChunkDraft>,
}

#[derive(Clone, Debug)]
pub struct WorkspaceIndex {
    pub root: PathBuf,
    files: Vec<IndexedFile>,
    chunks: Vec<IndexedChunk>,
    dims: usize,
    embedding_id: Option<String>,
}

impl Default for WorkspaceIndex {
    fn default() -> Self {
        Self {
            root: PathBuf::new(),
            files: Vec::new(),
            chunks: Vec::new(),
            dims: EMBED_DIMS,
            embedding_id: None,
        }
    }
}

pub fn chunk_lines(content: &str, max_chars: usize, overlap_ratio: f32) -> Vec<Chunk> {
    let max_chars = max_chars.max(1);
    let overlap_ratio = overlap_ratio.clamp(0.05, 0.35);
    let overlap_chars = (max_chars as f32 * overlap_ratio).floor() as usize;
    let lines: Vec<&str> = content.split('\n').collect();
    if lines.is_empty() {
        return vec![Chunk {
            start_line: 1,
            end_line: 1,
            text: content.chars().take(max_chars).collect(),
        }];
    }

    let mut chunks = Vec::new();
    let mut buf = String::new();
    let mut start_line = 1usize;
    for (index, line) in lines.iter().enumerate() {
        let line_no = index + 1;
        let candidate = if buf.is_empty() {
            (*line).to_string()
        } else {
            format!("{buf}\n{line}")
        };
        if candidate.len() > max_chars && !buf.is_empty() {
            chunks.push(Chunk {
                start_line,
                end_line: line_no - 1,
                text: buf.clone(),
            });
            let overlap = tail_overlap(&buf, overlap_chars);
            buf = if overlap.is_empty() {
                (*line).to_string()
            } else {
                format!("{overlap}\n{line}")
            };
            start_line = line_no.saturating_sub(count_lines(&buf).saturating_sub(1));
            start_line = start_line.max(1);
        } else {
            buf = candidate;
        }
    }
    if !buf.is_empty() {
        chunks.push(Chunk {
            start_line,
            end_line: lines.len(),
            text: buf,
        });
    }
    if chunks.is_empty() {
        chunks.push(Chunk {
            start_line: 1,
            end_line: 1,
            text: String::new(),
        });
    }
    chunks
}

pub fn collect(root: impl AsRef<Path>) -> CollectedWorkspace {
    let root = root.as_ref().to_path_buf();
    let mut files = Vec::new();
    collect_files(&root, &root, &mut files);
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let mut drafts = Vec::new();
    for file in &files {
        if drafts.len() >= MAX_CHUNKS {
            break;
        }
        for chunk in chunk_lines(&file.content, DEFAULT_CHUNK_CHARS, 0.18) {
            if drafts.len() >= MAX_CHUNKS {
                break;
            }
            let embed_source = format!(
                "File: {} lines {}-{}\n{}",
                file.path, chunk.start_line, chunk.end_line, chunk.text
            );
            drafts.push(ChunkDraft {
                path: file.path.clone(),
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                text: chunk.text,
                embed_source,
            });
        }
    }
    CollectedWorkspace {
        root,
        files,
        drafts,
    }
}

impl CollectedWorkspace {
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn chunk_count(&self) -> usize {
        self.drafts.len()
    }

    pub fn embed_char_count(&self) -> usize {
        self.drafts
            .iter()
            .map(|draft| draft.embed_source.len())
            .sum()
    }

    pub fn sample_paths(&self, limit: usize) -> Vec<String> {
        self.files
            .iter()
            .take(limit)
            .map(|file| file.path.clone())
            .collect()
    }

    pub fn embed_sources(&self) -> Vec<String> {
        self.drafts
            .iter()
            .map(|draft| draft.embed_source.clone())
            .collect()
    }

    pub fn exclude(mut self, prefixes: &[String]) -> Self {
        let prefixes: Vec<String> = prefixes
            .iter()
            .filter_map(|prefix| normalize_rel(prefix))
            .collect();
        if prefixes.is_empty() {
            return self;
        }
        let excluded = |path: &str| {
            prefixes
                .iter()
                .any(|prefix| path == prefix || path.starts_with(&format!("{prefix}/")))
        };
        self.files.retain(|file| !excluded(&file.path));
        self.drafts.retain(|draft| !excluded(&draft.path));
        self
    }

    pub fn finish(
        self,
        embeddings: Vec<Vec<f32>>,
        embedding_id: Option<String>,
    ) -> Result<WorkspaceIndex, String> {
        if embeddings.len() != self.drafts.len() {
            return Err(format!(
                "embedding count {} does not match chunk count {}",
                embeddings.len(),
                self.drafts.len()
            ));
        }
        let dims = embeddings.first().map(Vec::len).unwrap_or(EMBED_DIMS);
        if embeddings.iter().any(|vector| vector.len() != dims) {
            return Err("embedding dimensions are not consistent".into());
        }
        let chunks = self
            .drafts
            .into_iter()
            .zip(embeddings)
            .map(|(draft, embedding)| IndexedChunk {
                path: draft.path,
                start_line: draft.start_line,
                end_line: draft.end_line,
                text: draft.text,
                embedding,
            })
            .collect();
        Ok(WorkspaceIndex {
            root: self.root,
            files: self.files,
            chunks,
            dims,
            embedding_id,
        })
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct StoredFile {
    pub path: String,
    pub content: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct StoredChunk {
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub text: String,
    pub embedding: Vec<f32>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct IndexSnapshot {
    pub root: String,
    pub dims: usize,
    pub embedding_id: Option<String>,
    pub files: Vec<StoredFile>,
    pub chunks: Vec<StoredChunk>,
}

#[derive(Clone, Debug)]
pub struct ScopeNode {
    pub name: String,
    pub path: String,
    pub kind: &'static str,
}

pub fn list_scope(root: &Path, parent: &str) -> Result<Vec<ScopeNode>, String> {
    let parent = parent.trim();
    let parent = if parent.is_empty() {
        String::new()
    } else {
        normalize_rel(parent).ok_or_else(|| "workspace_not_resolved".to_string())?
    };
    let dir = if parent.is_empty() {
        root.to_path_buf()
    } else {
        root.join(&parent)
    };
    if !dir.is_dir() {
        return Err("workspace_not_resolved".into());
    }
    let mut entries = Vec::new();
    let read = fs::read_dir(&dir).map_err(|_| "workspace_not_resolved".to_string())?;
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let rel = if parent.is_empty() {
            name.clone()
        } else {
            format!("{parent}/{name}")
        };
        if should_skip_path(&rel) {
            continue;
        }
        if entry.path().is_dir() {
            entries.push(ScopeNode {
                name,
                path: rel,
                kind: "dir",
            });
        } else if is_indexable(&rel) {
            entries.push(ScopeNode {
                name,
                path: rel,
                kind: "file",
            });
        }
    }
    entries.sort_by(|left, right| {
        right.kind.cmp(left.kind).then_with(|| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        })
    });
    Ok(entries)
}

pub fn prepare(root: impl AsRef<Path>) -> WorkspaceIndex {
    prepare_hashed(root, EMBED_DIMS)
}

pub fn prepare_hashed(root: impl AsRef<Path>, dims: usize) -> WorkspaceIndex {
    let collected = collect(root);
    let dims = dims.max(8);
    let embeddings = collected
        .embed_sources()
        .iter()
        .map(|text| embed_text(text, dims))
        .collect();
    collected
        .finish(embeddings, None)
        .expect("hashed embeddings")
}

impl WorkspaceIndex {
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn dimensions(&self) -> usize {
        self.dims
    }

    pub fn embedding_id(&self) -> Option<&str> {
        self.embedding_id.as_deref()
    }

    pub fn uses_provider(&self) -> bool {
        self.embedding_id.is_some()
    }

    pub fn snapshot(&self) -> IndexSnapshot {
        IndexSnapshot {
            root: self.root.to_string_lossy().to_string(),
            dims: self.dims,
            embedding_id: self.embedding_id.clone(),
            files: self
                .files
                .iter()
                .map(|file| StoredFile {
                    path: file.path.clone(),
                    content: file.content.clone(),
                })
                .collect(),
            chunks: self
                .chunks
                .iter()
                .map(|chunk| StoredChunk {
                    path: chunk.path.clone(),
                    start_line: chunk.start_line,
                    end_line: chunk.end_line,
                    text: chunk.text.clone(),
                    embedding: chunk.embedding.clone(),
                })
                .collect(),
        }
    }

    pub fn from_snapshot(snapshot: IndexSnapshot) -> Self {
        let dims = snapshot
            .chunks
            .first()
            .map(|chunk| chunk.embedding.len())
            .unwrap_or(snapshot.dims.max(8));
        Self {
            root: PathBuf::from(snapshot.root),
            files: snapshot
                .files
                .into_iter()
                .map(|file| IndexedFile {
                    path: file.path,
                    content: file.content,
                })
                .collect(),
            chunks: snapshot
                .chunks
                .into_iter()
                .map(|chunk| IndexedChunk {
                    path: chunk.path,
                    start_line: chunk.start_line,
                    end_line: chunk.end_line,
                    text: chunk.text,
                    embedding: chunk.embedding,
                })
                .collect(),
            dims,
            embedding_id: snapshot.embedding_id,
        }
    }

    pub fn grep(&self, query: &str, limit: usize) -> Vec<GrepHit> {
        let query = query.trim();
        if query.is_empty() {
            return Vec::new();
        }
        let needle = query.to_ascii_lowercase();
        let limit = limit.clamp(1, 80);
        let mut hits = Vec::new();
        for file in &self.files {
            for (index, line) in file.content.lines().enumerate() {
                if line.to_ascii_lowercase().contains(&needle) {
                    hits.push(GrepHit {
                        path: file.path.clone(),
                        line: index + 1,
                        text: line.to_string(),
                    });
                    if hits.len() >= limit {
                        return hits;
                    }
                }
            }
        }
        hits
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchHit> {
        let query_vec = embed_text(query, self.dims.max(8));
        self.rank(query, &query_vec, limit)
    }

    pub fn rank(&self, query: &str, query_vec: &[f32], limit: usize) -> Vec<SearchHit> {
        let query = query.trim();
        if query.is_empty() {
            return Vec::new();
        }
        let limit = limit.clamp(1, 24);
        let mut scored: Vec<SearchHit> = self
            .chunks
            .iter()
            .map(|chunk| {
                let keyword = keyword_score(query, &chunk.path, &chunk.text);
                let semantic = cosine(query_vec, &chunk.embedding);
                SearchHit {
                    path: chunk.path.clone(),
                    start_line: chunk.start_line,
                    end_line: chunk.end_line,
                    text: chunk.text.clone(),
                    score: keyword * 0.35 + semantic * 0.65,
                }
            })
            .filter(|hit| hit.score > 0.0)
            .collect();
        scored.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.path.cmp(&right.path))
        });
        scored.truncate(limit);
        scored
    }

    pub fn read_file(&self, path: &str, offset: usize, length: usize) -> Option<FileSlice> {
        let path = normalize_rel(path)?;
        let file = self.files.iter().find(|file| file.path == path)?;
        let total = file.content.len();
        let offset = offset.min(total);
        let length = length.clamp(1, 8000).min(total.saturating_sub(offset));
        let content = file.content.get(offset..offset + length)?.to_string();
        Some(FileSlice {
            path,
            has_more: offset + length < total,
            offset,
            length: content.len(),
            total,
            content,
        })
    }
}

pub fn embed_text(text: &str, dims: usize) -> Vec<f32> {
    let dims = dims.max(8);
    let mut vector = vec![0.0f32; dims];
    let tokens = tokens(text);
    if tokens.is_empty() {
        return vector;
    }
    for token in tokens {
        let hash = fnv1a(&token);
        let index = (hash as usize) % dims;
        let sign = if (hash >> 8) & 1 == 0 { 1.0 } else { -1.0 };
        vector[index] += sign;
    }
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut vector {
            *value /= norm;
        }
    }
    vector
}

pub fn cosine(left: &[f32], right: &[f32]) -> f32 {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return 0.0;
    }
    left.iter()
        .zip(right.iter())
        .map(|(a, b)| a * b)
        .sum::<f32>()
        .clamp(-1.0, 1.0)
}

fn collect_files(root: &Path, dir: &Path, files: &mut Vec<IndexedFile>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    let mut names: Vec<_> = entries.filter_map(Result::ok).collect();
    names.sort_by_key(|entry| entry.file_name());
    for entry in names {
        let path = entry.path();
        let rel = match path.strip_prefix(root) {
            Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        if should_skip_path(&rel) {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, files);
            continue;
        }
        if !is_indexable(&rel) {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        if bytes.len() > MAX_FILE_BYTES || bytes.contains(&0) {
            continue;
        }
        let Ok(content) = String::from_utf8(bytes) else {
            continue;
        };
        files.push(IndexedFile { path: rel, content });
    }
}

fn should_skip_path(rel: &str) -> bool {
    let lower = rel.replace('\\', "/").to_ascii_lowercase();
    SKIP_DIRS.iter().any(|dir| {
        let dir = dir.to_ascii_lowercase();
        lower == dir || lower.starts_with(&format!("{dir}/")) || lower.contains(&format!("/{dir}/"))
    })
}

fn is_indexable(rel: &str) -> bool {
    if should_never_index(rel) {
        return false;
    }
    let lower = rel.replace('\\', "/").to_ascii_lowercase();
    EXTENSIONS
        .iter()
        .any(|ext| lower.ends_with(&format!(".{ext}")))
}

fn should_never_index(rel: &str) -> bool {
    let rel = rel.replace('\\', "/");
    let base = rel.rsplit('/').next().unwrap_or(&rel);
    if base == ".env" {
        return true;
    }
    if base == ".env.example" {
        return false;
    }
    if base.starts_with(".env.") {
        return true;
    }
    let lower = rel.to_ascii_lowercase();
    lower.ends_with(".env") && !lower.ends_with(".env.example")
}

fn normalize_rel(path: &str) -> Option<String> {
    let path = path.trim().replace('\\', "/");
    if path.is_empty() || path.starts_with('/') || path.contains('\0') {
        return None;
    }
    let mut out = Vec::new();
    for part in path.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return None;
        }
        out.push(part);
    }
    if out.is_empty() {
        None
    } else {
        Some(out.join("/"))
    }
}

fn tail_overlap(text: &str, overlap_chars: usize) -> String {
    if overlap_chars == 0 || text.is_empty() {
        return String::new();
    }
    if text.len() <= overlap_chars {
        return text.to_string();
    }
    let mut start = text.len().saturating_sub(overlap_chars);
    while start > 0 && !text.is_char_boundary(start) {
        start -= 1;
    }
    text[start..].to_string()
}

fn count_lines(text: &str) -> usize {
    if text.is_empty() {
        0
    } else {
        text.bytes().filter(|byte| *byte == b'\n').count() + 1
    }
}

fn tokens(text: &str) -> Vec<String> {
    text.to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .filter(|token| token.len() >= 2)
        .map(ToOwned::to_owned)
        .collect()
}

fn keyword_score(query: &str, path: &str, text: &str) -> f32 {
    let haystack = format!("{path} {text}").to_ascii_lowercase();
    let query_tokens = tokens(query);
    if query_tokens.is_empty() {
        return 0.0;
    }
    let hits = query_tokens
        .iter()
        .filter(|token| haystack.contains(token.as_str()))
        .count();
    hits as f32 / query_tokens.len() as f32
}

fn fnv1a(token: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in token.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
