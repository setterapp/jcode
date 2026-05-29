use crate::memory_graph::MemoryGraph;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

// === Graph Cache ===

/// Bound the number of cached memory graphs. Each entry clones a full
/// `MemoryGraph` (memories + tags + edges), so an unbounded cache grows with
/// every distinct project/global graph path loaded over the process lifetime.
const GRAPH_CACHE_MAX: usize = 8;

struct GraphCacheEntry {
    graph: MemoryGraph,
    modified: Option<SystemTime>,
}

struct GraphCache {
    entries: HashMap<PathBuf, GraphCacheEntry>,
    /// Insertion order for FIFO eviction when over `GRAPH_CACHE_MAX`.
    order: VecDeque<PathBuf>,
}

impl GraphCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }
}

static GRAPH_CACHE: OnceLock<Mutex<GraphCache>> = OnceLock::new();

fn graph_cache() -> &'static Mutex<GraphCache> {
    GRAPH_CACHE.get_or_init(|| Mutex::new(GraphCache::new()))
}

fn graph_mtime(path: &PathBuf) -> Option<SystemTime> {
    std::fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

pub(super) fn cached_graph(path: &PathBuf) -> Option<MemoryGraph> {
    let modified = graph_mtime(path);
    let cache = graph_cache().lock().ok()?;
    let entry = cache.entries.get(path)?;
    if entry.modified == modified {
        Some(entry.graph.clone())
    } else {
        None
    }
}

pub(super) fn cache_graph(path: PathBuf, graph: &MemoryGraph) {
    let modified = graph_mtime(&path);
    if let Ok(mut cache) = graph_cache().lock() {
        let is_new = !cache.entries.contains_key(&path);
        if is_new {
            // Evict oldest entries (FIFO) until there is room for the new one.
            while cache.entries.len() >= GRAPH_CACHE_MAX {
                match cache.order.pop_front() {
                    Some(victim) => {
                        cache.entries.remove(&victim);
                    }
                    None => break,
                }
            }
            cache.order.push_back(path.clone());
        }
        cache.entries.insert(
            path,
            GraphCacheEntry {
                graph: graph.clone(),
                modified,
            },
        );
    }
}
