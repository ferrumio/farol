use farol_core::search::{SearchEntry, build_index, write_to_site};

fn entry(url: &str, title: &str, body: &str) -> SearchEntry {
    SearchEntry { url: url.into(), title: title.into(), section: None, body: body.into() }
}

#[test]
fn empty_index_works() {
    let idx = build_index(&[]).unwrap();
    assert!(idx.docs.is_empty());
    assert!(idx.index.is_empty());
}

#[test]
fn indexes_words_with_stemming() {
    let entries = vec![
        entry("/a/", "Running", "We are running every day"),
        entry("/b/", "Walking", "He walks slowly"),
    ];
    let idx = build_index(&entries).unwrap();
    assert_eq!(idx.docs.len(), 2);
    assert!(
        idx.index.contains_key("run"),
        "missing `run`: {:?}",
        idx.index.keys().collect::<Vec<_>>()
    );
}

#[test]
fn ranking_title_over_body() {
    let entries = vec![
        entry("/a/", "Rust tutorial", "beginners guide"),
        entry("/b/", "Other", "Rust appears only in body"),
    ];
    let idx = build_index(&entries).unwrap();
    let postings = idx.index.get("rust").expect("rust token");
    assert_eq!(postings.first().unwrap().doc, 0);
}

#[test]
fn snippet_is_truncated() {
    let body = "a".repeat(500);
    let entries = vec![entry("/a/", "t", &body)];
    let idx = build_index(&entries).unwrap();
    assert!(idx.docs[0].snippet.chars().count() < 200);
    assert!(idx.docs[0].snippet.ends_with('…'));
}

#[test]
fn write_creates_both_files() {
    let tmp = tempfile::TempDir::new().unwrap();
    let idx = build_index(&[entry("/a/", "hi", "hello world")]).unwrap();
    write_to_site(tmp.path(), &idx).unwrap();
    assert!(tmp.path().join("assets/search/docs.json").exists());
    assert!(tmp.path().join("assets/search/index.json").exists());
}
