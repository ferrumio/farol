use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tempfile::TempDir;

use farol_core::{Cache, Graph, Hash, Hasher, Node, Result};

struct Counting {
    id: String,
    hash: Hash,
    counter: Arc<AtomicUsize>,
    output: Vec<u8>,
}

impl Node for Counting {
    fn id(&self) -> &str {
        &self.id
    }
    fn input_hash(&self) -> Hash {
        self.hash
    }
    fn execute(&self) -> Result<Vec<u8>> {
        self.counter.fetch_add(1, Ordering::Relaxed);
        Ok(self.output.clone())
    }
}

fn node(id: &str, content: &[u8], counter: Arc<AtomicUsize>) -> Counting {
    Counting {
        id: id.to_string(),
        hash: Hasher::new().update(content).finish(),
        counter,
        output: content.to_vec(),
    }
}

#[test]
fn runs_all_nodes_on_cold() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut graph = Graph::new();
    graph.push(node("a", b"a", counter.clone()));
    graph.push(node("b", b"b", counter.clone()));
    graph.push(node("c", b"c", counter.clone()));

    let report = graph.execute(None).unwrap();
    assert_eq!(report.cache_misses, 3);
    assert_eq!(report.cache_hits, 0);
    assert_eq!(counter.load(Ordering::Relaxed), 3);
}

#[test]
fn warm_run_hits_cache() {
    let tmp = TempDir::new().unwrap();
    let cache = Cache::open(tmp.path().join("c.redb")).unwrap();
    let counter = Arc::new(AtomicUsize::new(0));

    {
        let mut g = Graph::new();
        g.push(node("a", b"a", counter.clone()));
        g.push(node("b", b"b", counter.clone()));
        g.execute(Some(&cache)).unwrap();
    }
    assert_eq!(counter.load(Ordering::Relaxed), 2);

    let warm_counter = Arc::new(AtomicUsize::new(0));
    let mut g = Graph::new();
    g.push(node("a", b"a", warm_counter.clone()));
    g.push(node("b", b"b", warm_counter.clone()));
    let report = g.execute(Some(&cache)).unwrap();
    assert_eq!(report.cache_hits, 2);
    assert_eq!(report.cache_misses, 0);
    assert_eq!(warm_counter.load(Ordering::Relaxed), 0);
}

#[test]
fn changed_input_invalidates_entry() {
    let tmp = TempDir::new().unwrap();
    let cache = Cache::open(tmp.path().join("c.redb")).unwrap();

    let c1 = Arc::new(AtomicUsize::new(0));
    {
        let mut g = Graph::new();
        g.push(node("a", b"v1", c1.clone()));
        g.execute(Some(&cache)).unwrap();
    }
    assert_eq!(c1.load(Ordering::Relaxed), 1);

    let c2 = Arc::new(AtomicUsize::new(0));
    let mut g = Graph::new();
    g.push(node("a", b"v2", c2.clone()));
    let report = g.execute(Some(&cache)).unwrap();
    assert_eq!(report.cache_misses, 1);
    assert_eq!(c2.load(Ordering::Relaxed), 1);
}

#[test]
fn hit_rate_is_computed() {
    let tmp = TempDir::new().unwrap();
    let cache = Cache::open(tmp.path().join("c.redb")).unwrap();
    let counter = Arc::new(AtomicUsize::new(0));

    {
        let mut g = Graph::new();
        g.push(node("a", b"a", counter.clone()));
        g.push(node("b", b"b", counter.clone()));
        g.execute(Some(&cache)).unwrap();
    }

    let warm = Arc::new(AtomicUsize::new(0));
    let mut g = Graph::new();
    g.push(node("a", b"a", warm.clone()));
    g.push(node("c", b"c", warm.clone()));
    let report = g.execute(Some(&cache)).unwrap();
    assert_eq!(report.cache_hits, 1);
    assert_eq!(report.cache_misses, 1);
    assert!((report.hit_rate() - 0.5).abs() < 1e-9);
}
