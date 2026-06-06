use std::path::{Path, PathBuf};

use redb::{Database, ReadableDatabase, TableDefinition};

use crate::error::{FarolError, Result};

/// The on-disk schema version. Bumping this clears cached data on load.
const SCHEMA_VERSION: u32 = 1;

/// Persistent key-value cache keyed by node id + input hash.
///
/// Layout:
/// - `meta` table stores a schema-version marker; mismatches cause full reset.
/// - `nodes` table stores `(node_id, input_hash_bytes) -> output_bytes`.
pub struct Cache {
    db: Database,
    path: PathBuf,
}

const META: TableDefinition<&str, u32> = TableDefinition::new("meta");
const NODES: TableDefinition<&[u8], &[u8]> = TableDefinition::new("nodes");
const SCHEMA_KEY: &str = "schema_version";

impl Cache {
    /// Open (or create) a cache at `path`. On schema mismatch, the cache is
    /// cleared automatically so users never need to manage this manually.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| FarolError::io(parent, e))?;
        }

        let db = Database::create(&path).map_err(|e| FarolError::cache(format!("open: {e}")))?;
        let cache = Self { db, path };
        cache.migrate()?;
        Ok(cache)
    }

    fn migrate(&self) -> Result<()> {
        let stored = {
            let txn = self
                .db
                .begin_read()
                .map_err(|e| FarolError::cache(format!("migrate read: {e}")))?;
            match txn.open_table(META) {
                Ok(table) => table
                    .get(SCHEMA_KEY)
                    .map_err(|e| FarolError::cache(format!("migrate meta: {e}")))?
                    .map(|v| v.value()),
                Err(_) => None,
            }
        };

        if stored == Some(SCHEMA_VERSION) {
            return Ok(());
        }

        // Either fresh DB or version mismatch - clear and rewrite.
        let txn =
            self.db.begin_write().map_err(|e| FarolError::cache(format!("migrate write: {e}")))?;
        {
            if txn.open_table(NODES).is_ok() {
                txn.delete_table(NODES)
                    .map_err(|e| FarolError::cache(format!("drop nodes: {e}")))?;
            }
            let mut meta =
                txn.open_table(META).map_err(|e| FarolError::cache(format!("open meta: {e}")))?;
            meta.insert(SCHEMA_KEY, SCHEMA_VERSION)
                .map_err(|e| FarolError::cache(format!("write meta: {e}")))?;
        }
        txn.commit().map_err(|e| FarolError::cache(format!("migrate commit: {e}")))?;
        Ok(())
    }

    /// Clear all cached entries but keep the schema marker.
    pub fn clear(&self) -> Result<()> {
        let txn =
            self.db.begin_write().map_err(|e| FarolError::cache(format!("clear write: {e}")))?;
        {
            if txn.open_table(NODES).is_ok() {
                txn.delete_table(NODES)
                    .map_err(|e| FarolError::cache(format!("drop nodes: {e}")))?;
            }
        }
        txn.commit().map_err(|e| FarolError::cache(format!("clear commit: {e}")))?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Look up a cached output by `(node_id, input_hash)`.
    pub fn get(&self, node_id: &str, input_hash: &[u8]) -> Result<Option<Vec<u8>>> {
        let key = make_key(node_id, input_hash);
        let txn = self.db.begin_read().map_err(|e| FarolError::cache(format!("get read: {e}")))?;
        let table = match txn.open_table(NODES) {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };
        let value = table
            .get(key.as_slice())
            .map_err(|e| FarolError::cache(format!("get: {e}")))?
            .map(|v| v.value().to_vec());
        Ok(value)
    }

    /// Store an output under `(node_id, input_hash)`.
    pub fn put(&self, node_id: &str, input_hash: &[u8], output: &[u8]) -> Result<()> {
        let key = make_key(node_id, input_hash);
        let txn =
            self.db.begin_write().map_err(|e| FarolError::cache(format!("put write: {e}")))?;
        {
            let mut table =
                txn.open_table(NODES).map_err(|e| FarolError::cache(format!("open nodes: {e}")))?;
            table
                .insert(key.as_slice(), output)
                .map_err(|e| FarolError::cache(format!("insert: {e}")))?;
        }
        txn.commit().map_err(|e| FarolError::cache(format!("put commit: {e}")))?;
        Ok(())
    }
}

fn make_key(node_id: &str, input_hash: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(node_id.len() + 1 + input_hash.len());
    key.extend_from_slice(node_id.as_bytes());
    key.push(0u8);
    key.extend_from_slice(input_hash);
    key
}

impl FarolError {
    pub(crate) fn cache(msg: impl Into<String>) -> Self {
        FarolError::Cache { message: msg.into() }
    }
}
