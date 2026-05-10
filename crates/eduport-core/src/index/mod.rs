//! SQLite + FTS5 search/filter index for eduport-core.
//!
//! The index is a derived cache over the markdown vault. Every row in
//! the database is reproducible from the on-disk entity files via
//! [`crate::EntityStore`] and vaultdb-core's parsed records — there is
//! no canonical state here that isn't also on disk.
//!
//! ## Layered API
//!
//! Three thin layers, each independently usable:
//!
//! 1. [`schema::init_schema`] — apply DDL + version migrations to a
//!    `rusqlite::Connection`. The lowest layer.
//! 2. [`writer`] / [`reader`] — free functions taking `&Connection`
//!    that mutate or query the index. Composing them into a single
//!    transaction is up to the caller (the watcher batch path will
//!    want this; one-off queries don't need it).
//! 3. [`Index`] — a thin struct that owns a `Connection` for the
//!    common single-process case where the caller doesn't care about
//!    transaction composition.
//!
//! ## Why free functions, not just methods on [`Index`]
//!
//! The Python sidecar's `eduport.index` was free functions over a
//! `sqlite3.Connection` precisely so the watcher's debounced-batch
//! path could open a transaction and call `upsert_entity` /
//! `delete_entity` repeatedly inside it. The Rust port preserves that
//! affordance: the watcher (Phase 8) will use `conn.transaction()`
//! and pass the resulting `Transaction` (which `Deref`s to `&Connection`
//! through rusqlite's API, via `&*tx`). Putting everything on `Index`
//! would have boxed callers into one-call-per-transaction.

pub mod reader;
pub mod reconcile;
pub mod schema;
pub mod writer;

pub use reader::{
    EntitySummary, PropertyFilter, PropertyValueCount, SearchHit,
    filter_entities_by_properties, list_entities, property_value_counts, search_fts,
};
pub use reconcile::{ReconcileSummary, reconcile};
pub use schema::{INDEX_SCHEMA_VERSION, InitOutcome, init_schema};
pub use writer::{
    clear_parse_error, delete_entity, record_parse_error, reindex_all_properties, upsert_entity,
};

use rusqlite::Connection;
use std::path::Path;

/// Convenience wrapper that owns a [`Connection`] with the schema
/// initialised. Use [`Index::conn`] / [`Index::conn_mut`] when you
/// need to drop down to the free-function API for transaction control.
pub struct Index {
    conn: Connection,
}

impl Index {
    /// Open or create the index database at `path`. The parent
    /// directory must already exist; this matches vaultdb-core's
    /// convention of "the caller decides where things live".
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        // Foreign-key enforcement is off by default in SQLite;
        // turn it on so `ON DELETE CASCADE` rows in `entity_tags` /
        // `properties` are actually removed when the parent
        // `entities` row goes.
        conn.execute("PRAGMA foreign_keys = ON", [])?;
        let _ = init_schema(&conn)?;
        Ok(Self { conn })
    }

    /// Open an in-memory index. Used in tests and by ephemeral
    /// reconcile flows that want a fresh index per call.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute("PRAGMA foreign_keys = ON", [])?;
        let _ = init_schema(&conn)?;
        Ok(Self { conn })
    }

    /// Borrow the underlying connection. Use this to compose multiple
    /// writer/reader calls in a single transaction.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Mutable borrow of the underlying connection. Required for
    /// `conn.transaction()`.
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
}

/// Crate-level error type for the index module. Wraps the underlying
/// `rusqlite::Error` and the eduport-core domain error so callers
/// don't need to thread two error types through every signature.
#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),

    #[error(transparent)]
    Eduport(#[from] crate::EduportError),

    #[error("index data error: {0}")]
    Data(String),
}

pub type Result<T> = std::result::Result<T, IndexError>;
