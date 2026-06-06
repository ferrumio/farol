use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use rayon::prelude::*;

use crate::{cache::Cache, error::Result, hash::Hash};

/// A unit of cacheable work.
///
/// Each node:
/// - has a stable `id` (used as part of the cache key)
/// - reports the hash of its inputs (when inputs haven't changed, we skip)
/// - produces an owned byte payload when executed (can be empty for side-effect-only nodes)
///
/// Side-effect nodes (writing files, copying assets) can just return `vec![]`
/// after doing their work; the cache hit on an empty value is the "skip"
/// signal. Nodes that produce reusable data (a rendered HTML blob) should
/// return that data so a warm rebuild can restore the artifact.
pub trait Node: Send + Sync {
    fn id(&self) -> &str;
    fn input_hash(&self) -> Hash;
    fn execute(&self) -> Result<Vec<u8>>;

    /// Invoked after a cache hit. Gives side-effect nodes a chance to restore
    /// from cached data (e.g. rewrite the output file). Default: no-op.
    fn restore(&self, _cached: &[u8]) -> Result<()> {
        Ok(())
    }
}

/// A dependency graph of nodes. For v0.1 the execution model is a single
/// "wave": every registered node is independent and can run in parallel. More
/// complex dependency edges are a v0.2 feature (#nn).
pub struct Graph {
    nodes: Vec<Box<dyn Node>>,
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

impl Graph {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn push<N: Node + 'static>(&mut self, node: N) {
        self.nodes.push(Box::new(node));
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Execute every node in parallel, consulting `cache` when provided.
    ///
    /// Returns a `Report` with per-node timing and cache stats.
    pub fn execute(self, cache: Option<&Cache>) -> Result<Report> {
        let started = Instant::now();
        let hits = Arc::new(AtomicUsize::new(0));
        let misses = Arc::new(AtomicUsize::new(0));
        let timings = Arc::new(Mutex::new(Vec::new()));

        let first_error = Arc::new(Mutex::new(None));

        self.nodes.into_par_iter().for_each(|node| {
            if first_error.lock().unwrap().is_some() {
                return;
            }
            let start = Instant::now();
            let id = node.id().to_string();
            let input_hash = node.input_hash();

            let cached = cache.and_then(|c| c.get(&id, input_hash.as_bytes()).ok().flatten());

            let outcome = if let Some(bytes) = cached.as_deref() {
                hits.fetch_add(1, Ordering::Relaxed);
                node.restore(bytes).map(|_| NodeOutcome::Hit)
            } else {
                misses.fetch_add(1, Ordering::Relaxed);
                match node.execute() {
                    Ok(output) => {
                        if let Some(c) = cache {
                            if let Err(e) = c.put(&id, input_hash.as_bytes(), &output) {
                                tracing::warn!(node = %id, error = %e, "failed to write cache");
                            }
                        }
                        Ok(NodeOutcome::Miss)
                    }
                    Err(e) => Err(e),
                }
            };

            let elapsed = start.elapsed();
            match outcome {
                Ok(o) => timings.lock().unwrap().push(NodeTiming { id, outcome: o, elapsed }),
                Err(e) => {
                    let mut slot = first_error.lock().unwrap();
                    if slot.is_none() {
                        *slot = Some(e);
                    }
                }
            }
        });

        if let Some(e) =
            Arc::try_unwrap(first_error).ok().and_then(|m| m.into_inner().ok()).flatten()
        {
            return Err(e);
        }

        let mut timings =
            Arc::try_unwrap(timings).ok().and_then(|m| m.into_inner().ok()).unwrap_or_default();
        timings.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(Report {
            total_elapsed: started.elapsed(),
            cache_hits: hits.load(Ordering::Relaxed),
            cache_misses: misses.load(Ordering::Relaxed),
            timings,
        })
    }
}

/// Whether a node ran or hit the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeOutcome {
    Hit,
    Miss,
}

/// Timing data for a single node.
#[derive(Debug, Clone)]
pub struct NodeTiming {
    pub id: String,
    pub outcome: NodeOutcome,
    pub elapsed: Duration,
}

/// Aggregate report of a graph execution.
#[derive(Debug, Clone)]
pub struct Report {
    pub total_elapsed: Duration,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub timings: Vec<NodeTiming>,
}

impl Report {
    pub fn total_nodes(&self) -> usize {
        self.cache_hits + self.cache_misses
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.total_nodes();
        if total == 0 { 0.0 } else { self.cache_hits as f64 / total as f64 }
    }
}
