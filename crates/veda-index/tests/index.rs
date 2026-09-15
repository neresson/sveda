use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use veda_index::{chunk_lines, cosine, embed_text, prepare};

fn unique_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("veda-index-{tag}-{nanos}"));
    fs::create_dir_all(&dir).expect("temp");
    dir
}

fn write_workspace() -> PathBuf {
    let root = unique_dir("ws");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("node_modules")).unwrap();
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::write(
        root.join("src/hello.rs"),
        "fn greet() {\n    println!(\"hello from veda\");\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("src/long.rs"),
        (0..80)
            .map(|i| format!("line_{i:03} keep_this_token_{i}\n"))
            .collect::<String>(),
    )
    .unwrap();
    fs::write(root.join("node_modules/skip.js"), "secret_from_vendor").unwrap();
    fs::write(root.join(".git/config"), "gitdir").unwrap();
    fs::write(root.join(".env"), "SECRET=super-secret").unwrap();
    fs::write(root.join("notes.md"), "searchable markdown about occupancy").unwrap();
    root
}

#[test]
fn chunk_lines_splits_on_budget_and_keeps_overlap() {
    let content = (0..40)
        .map(|i| format!("aaaaaaaaaa line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let chunks = chunk_lines(&content, 120, 0.18);
    assert!(chunks.len() >= 2);
    assert!(chunks[0].start_line == 1);
    assert!(chunks[0].text.len() <= 120);
    assert!(chunks.last().unwrap().end_line >= chunks[0].end_line);
    let joined: String = chunks.iter().map(|chunk| chunk.text.clone()).collect();
    assert!(joined.contains("line 0"));
    assert!(joined.contains("line 39"));
}

#[test]
fn prepare_skips_vendor_git_and_env_secrets() {
    let root = write_workspace();
    let index = prepare(&root);
    assert!(index.file_count() >= 2);
    let paths = format!("{:?}", index);
    assert!(!paths.contains("node_modules"));
    assert!(!paths.contains(".env"));
    let grep = index.grep("secret_from_vendor", 10);
    assert!(grep.is_empty());
    let env_hits = index.grep("super-secret", 10);
    assert!(env_hits.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn grep_finds_source_hits_with_line_numbers() {
    let root = write_workspace();
    let index = prepare(&root);
    let hits = index.grep("hello from veda", 8);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "src/hello.rs");
    assert_eq!(hits[0].line, 2);
    assert!(hits[0].text.contains("hello from veda"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn search_ranks_relevant_chunk_above_unrelated() {
    let root = write_workspace();
    let index = prepare(&root);
    let hits = index.search("occupancy markdown", 5);
    assert!(!hits.is_empty());
    assert_eq!(hits[0].path, "notes.md");
    assert!(hits[0].score > 0.0);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn read_file_slices_content() {
    let root = write_workspace();
    let index = prepare(&root);
    let slice = index.read_file("src/hello.rs", 0, 8).expect("file");
    assert_eq!(slice.path, "src/hello.rs");
    assert_eq!(slice.content, "fn greet");
    assert!(slice.has_more);
    assert_eq!(slice.offset, 0);
    assert_eq!(slice.length, 8);
    let rest = index.read_file("src/hello.rs", 8, 4000).expect("rest");
    assert!(rest.content.starts_with("("));
    assert!(!rest.has_more);
    assert!(index.read_file("../outside.rs", 0, 10).is_none());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn hashed_embeddings_are_normalized_and_similar_for_related_text() {
    let left = embed_text("search occupancy limits redis semaphore", 256);
    let right = embed_text("redis occupancy semaphore for streams", 256);
    let unrelated = embed_text("banana smoothie recipe", 256);
    assert_eq!(left.len(), 256);
    let norm: f32 = left.iter().map(|v| v * v).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 0.02);
    assert!(cosine(&left, &right) > cosine(&left, &unrelated));
}
