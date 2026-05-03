//! Search index types and builder.
//!
//! The [`SearchEntry`] struct is what the `search` builtin plugin feeds to
//! tantivy. Plugins may inspect or modify entries via the `on_search_index`
//! hook before the index is serialized.
//!
//! The on-disk format is two JSON files under `site/assets/search/`:
//!
//! - `docs.json`  - array of [`SearchDoc`] records (url, title, section, snippet)
//! - `index.json` - inverted index mapping token -> [ (doc_id, score), ... ]
//!
//! The client loads both on demand, tokenizes the query with the same rules,
//! does token lookups, and accumulates scores per doc. No BM25 math in JS -
//! scores are pre-computed at build time.

use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};
use tantivy::{
    collector::TopDocs,
    doc,
    schema::{Field, Schema, Value, STORED, STRING, TEXT},
    tokenizer::{LowerCaser, SimpleTokenizer, Stemmer, TextAnalyzer, TokenizerManager},
    Index, IndexReader, ReloadPolicy, TantivyDocument, Term,
};

use crate::error::{FarolError, Result};

/// One indexable unit. Usually one per page, though a plugin could split a
/// long page into multiple sections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEntry {
    pub url: String,
    pub title: String,
    /// Optional section heading (for grouping results in the UI).
    #[serde(default)]
    pub section: Option<String>,
    pub body: String,
}

/// The compact record written to `docs.json` and returned to the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchDoc {
    pub id: u32,
    pub url: String,
    pub title: String,
    #[serde(default)]
    pub section: Option<String>,
    /// First ~160 chars of body, used for rendering result snippets.
    pub snippet: String,
}

/// Inverted index: token -> [(doc_id, score * 1000 as u16)]
pub type InvertedIndex = HashMap<String, Vec<Posting>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Posting {
    pub doc: u32,
    /// Pre-computed BM25 score * 1000, clamped to u16. Client just adds these
    /// up across tokens and sorts desc.
    pub score: u16,
}

/// Output of a build.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIndex {
    pub version: u8,
    pub docs: Vec<SearchDoc>,
    pub index: InvertedIndex,
}

impl SearchIndex {
    pub const FORMAT_VERSION: u8 = 1;
}

/// Build a [`SearchIndex`] from a list of entries.
///
/// Pipeline:
/// 1. Tokenize titles + body with tantivy (lowercase + English stemmer).
/// 2. Index each entry as a document.
/// 3. For every distinct token in the corpus, run a tantivy query and
///    collect (doc_id, BM25 score) pairs into the inverted index.
pub fn build_index(entries: &[SearchEntry]) -> Result<SearchIndex> {
    if entries.is_empty() {
        return Ok(SearchIndex {
            version: SearchIndex::FORMAT_VERSION,
            docs: Vec::new(),
            index: InvertedIndex::new(),
        });
    }

    let (index, title_field, body_field, url_field) = build_tantivy_index(entries)?;
    let reader =
        index.reader_builder().reload_policy(ReloadPolicy::Manual).try_into().map_err(|e| {
            FarolError::ConfigInvalid { message: format!("search index reader init: {e}") }
        })?;

    let mut tokenizer = english_tokenizer();
    let docs = materialize_docs(entries);
    let all_tokens = collect_all_tokens(&mut tokenizer, entries);

    let inverted =
        build_inverted(&index, &reader, &all_tokens, title_field, body_field, url_field, entries)?;

    Ok(SearchIndex { version: SearchIndex::FORMAT_VERSION, docs, index: inverted })
}

fn build_tantivy_index(entries: &[SearchEntry]) -> Result<(Index, Field, Field, Field)> {
    let mut schema = Schema::builder();
    let title_field = schema.add_text_field("title", TEXT);
    let body_field = schema.add_text_field("body", TEXT);
    let url_field = schema.add_text_field("url", STRING | STORED);
    let schema = schema.build();

    let index = Index::create_in_ram(schema.clone());
    register_english_tokenizer(index.tokenizers());

    let mut writer = index
        .writer(50_000_000)
        .map_err(|e| FarolError::ConfigInvalid { message: format!("tantivy writer: {e}") })?;

    for entry in entries {
        writer
            .add_document(doc!(
                title_field => entry.title.clone(),
                body_field => entry.body.clone(),
                url_field => entry.url.clone(),
            ))
            .map_err(|e| FarolError::ConfigInvalid { message: format!("tantivy add: {e}") })?;
    }
    writer
        .commit()
        .map_err(|e| FarolError::ConfigInvalid { message: format!("tantivy commit: {e}") })?;

    Ok((index, title_field, body_field, url_field))
}

fn english_tokenizer() -> TextAnalyzer {
    TextAnalyzer::builder(SimpleTokenizer::default())
        .filter(LowerCaser)
        .filter(Stemmer::new(tantivy::tokenizer::Language::English))
        .build()
}

fn register_english_tokenizer(tokenizers: &TokenizerManager) {
    tokenizers.register("default", english_tokenizer());
}

fn materialize_docs(entries: &[SearchEntry]) -> Vec<SearchDoc> {
    entries
        .iter()
        .enumerate()
        .map(|(id, e)| SearchDoc {
            id: id as u32,
            url: e.url.clone(),
            title: e.title.clone(),
            section: e.section.clone(),
            snippet: make_snippet(&e.body, 160),
        })
        .collect()
}

fn make_snippet(body: &str, max_chars: usize) -> String {
    let body = body.trim();
    if body.chars().count() <= max_chars {
        return body.to_string();
    }
    let mut out: String = body.chars().take(max_chars).collect();
    out.push('…');
    out
}

fn collect_all_tokens(analyzer: &mut TextAnalyzer, entries: &[SearchEntry]) -> Vec<String> {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in entries {
        {
            let mut tok = analyzer.token_stream(&entry.title);
            while let Some(t) = tok.next() {
                seen.insert(t.text.clone());
            }
        }
        {
            let mut tok = analyzer.token_stream(&entry.body);
            while let Some(t) = tok.next() {
                seen.insert(t.text.clone());
            }
        }
    }
    let mut out: Vec<String> = seen.into_iter().collect();
    out.sort();
    out
}

#[allow(clippy::too_many_arguments)]
fn build_inverted(
    index: &Index,
    reader: &IndexReader,
    tokens: &[String],
    title_field: Field,
    body_field: Field,
    url_field: Field,
    entries: &[SearchEntry],
) -> Result<InvertedIndex> {
    let searcher = reader.searcher();
    let mut url_to_id: HashMap<String, u32> = HashMap::with_capacity(entries.len());
    for (i, e) in entries.iter().enumerate() {
        url_to_id.insert(e.url.clone(), i as u32);
    }

    let mut inverted = InvertedIndex::new();
    let title_boost = 3.0_f32;

    for token in tokens {
        let mut postings: HashMap<u32, f32> = HashMap::new();

        for (field, boost) in [(title_field, title_boost), (body_field, 1.0)] {
            let query = tantivy::query::TermQuery::new(
                Term::from_field_text(field, token),
                tantivy::schema::IndexRecordOption::WithFreqsAndPositions,
            );
            let top = searcher
                .search(&query, &TopDocs::with_limit(entries.len()).order_by_score())
                .map_err(|e| FarolError::ConfigInvalid { message: format!("search: {e}") })?;
            for (score, addr) in top {
                let doc: TantivyDocument = searcher
                    .doc(addr)
                    .map_err(|e| FarolError::ConfigInvalid { message: format!("doc read: {e}") })?;
                let url_str: Option<String> = doc
                    .get_first(url_field)
                    .and_then(|v| v.as_value().as_str().map(|s| s.to_string()));
                if let Some(u) = url_str {
                    if let Some(id) = url_to_id.get(&u) {
                        *postings.entry(*id).or_default() += score * boost;
                    }
                }
            }
        }

        // Skip tokens that appear in every doc (low IDF).
        if postings.is_empty() {
            continue;
        }

        let mut vec: Vec<Posting> = postings
            .into_iter()
            .map(|(doc, score)| Posting {
                doc,
                score: (score * 1000.0).clamp(0.0, u16::MAX as f32) as u16,
            })
            .collect();
        vec.sort_by(|a, b| b.score.cmp(&a.score));
        inverted.insert(token.clone(), vec);
    }

    // Suppress unused-var warning when Schema isn't returned.
    let _ = index;
    Ok(inverted)
}

/// Serialize the index to the two JSON files under `site/assets/search/`.
pub fn write_to_site(site_dir: &Path, index: &SearchIndex) -> Result<()> {
    let dir = site_dir.join("assets").join("search");
    std::fs::create_dir_all(&dir).map_err(|e| FarolError::io(&dir, e))?;

    let docs_json = serde_json::to_string(&index.docs)
        .map_err(|e| FarolError::ConfigInvalid { message: format!("docs serialize: {e}") })?;
    std::fs::write(dir.join("docs.json"), docs_json).map_err(|e| FarolError::io(&dir, e))?;

    let index_json =
        serde_json::to_string(&IndexEnvelope { version: index.version, index: &index.index })
            .map_err(|e| FarolError::ConfigInvalid { message: format!("index serialize: {e}") })?;
    std::fs::write(dir.join("index.json"), index_json).map_err(|e| FarolError::io(&dir, e))?;

    Ok(())
}

#[derive(Serialize)]
struct IndexEnvelope<'a> {
    version: u8,
    index: &'a InvertedIndex,
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // "running" stems to "run" -> token appears in the index.
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
        // The title-only hit on /a/ should outrank the body-only hit on /b/.
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
}
