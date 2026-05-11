//! Thin wrapper over `vaultdb-fts` that adapts the generic crate to
//! eduport's domain shape (typed entities, `eduport-type/<value>`
//! tag discriminator, custom-property prose into the FTS5
//! `custom_text` column).
//!
//! The bespoke SQLite + FTS5 implementation that used to live here
//! moved into `vaultdb-fts` so other consumers can reuse it. This
//! module is the **eduport-specific adapter** — projection closures,
//! `SearchHit` extension (we add `entity_type` since vaultdb-fts
//! treats type as just-another-tag), and the entity-aware upsert
//! signature.
//!
//! The legacy `Index` type alias still resolves so tauri-side call
//! sites don't all churn at once.

pub mod reader;
pub mod reconcile;
pub mod writer;

pub use reader::{SearchHit, search_fts};
pub use reconcile::{ReconcileSummary, reconcile};
pub use vaultdb_fts::{FTS_SCHEMA_VERSION as INDEX_SCHEMA_VERSION, InitOutcome};
pub use writer::{
    clear_parse_error, delete_entity, record_parse_error, upsert_entity,
};

use rusqlite::Connection;
use std::path::Path;

/// Convenience wrapper that owns a [`Connection`] with the FTS5
/// schema initialised. Thin shim over [`vaultdb_fts::FtsIndex`] —
/// preserves the eduport-facing API while delegating storage to the
/// shared crate.
pub struct Index {
    inner: vaultdb_fts::FtsIndex,
}

impl Index {
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            inner: vaultdb_fts::FtsIndex::open(path)?,
        })
    }

    pub fn open_in_memory() -> Result<Self> {
        Ok(Self {
            inner: vaultdb_fts::FtsIndex::open_in_memory()?,
        })
    }

    pub fn conn(&self) -> &Connection {
        self.inner.conn()
    }

    pub fn conn_mut(&mut self) -> &mut Connection {
        self.inner.conn_mut()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),

    #[error(transparent)]
    Vaultdb(#[from] vaultdb_core::VaultdbError),

    #[error(transparent)]
    Eduport(#[from] crate::EduportError),

    #[error("index data error: {0}")]
    Data(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<vaultdb_fts::FtsError> for IndexError {
    fn from(e: vaultdb_fts::FtsError) -> Self {
        match e {
            vaultdb_fts::FtsError::Sqlite(s) => IndexError::Sqlite(s),
            vaultdb_fts::FtsError::Vault(v) => IndexError::Vaultdb(v),
            vaultdb_fts::FtsError::Data(d) => IndexError::Data(d),
            vaultdb_fts::FtsError::Io(io) => IndexError::Io(io),
        }
    }
}

pub type Result<T> = std::result::Result<T, IndexError>;
