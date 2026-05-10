//! # eduport-core
//!
//! Eduport's domain layer, sitting on top of [`vaultdb-core`]. Owns the
//! concerns the spec assigns to it (per
//! `docs/superpowers/specs/2026-05-09-vaultdb-rewrite-design.md`):
//!
//! - The 8 entity types and their typed property schemas
//! - Validation against those schemas
//! - The FTS5 search index (built over vaultdb-core's parsed records)
//! - File watcher (notify-based) emitting typed `VaultEvent`s
//! - EML → Email-entity import
//! - Saved-views storage (`.eduport/views.yaml`)
//! - Settings persistence (`.eduport/settings.yaml` / `data folder`)
//!
//! ## What lives here vs. in vaultdb-core
//!
//! Anything that's specific to eduport's domain (entity types, FTS5,
//! watcher, EML) lives here. Anything that's generic markdown-vault
//! infrastructure (parse, query, link graph, mutation builders, atomic
//! writes, journal-based crash recovery) lives in vaultdb-core. This
//! crate path-deps vaultdb-core during co-development; once vaultdb-core
//! ships v1.0.0 to crates.io we'll switch to a versioned dep.
//!
//! ## Phase status
//!
//! This is the Phase 4 scaffold — only the crate exists, no real
//! functionality is ported yet. Subsequent phases populate the
//! modules listed above. See the rewrite spec for the migration order.

#![forbid(unsafe_code)]

pub mod eml;
pub mod entity;
pub mod entity_type;
pub mod index;
pub mod schema;
pub mod settings;
pub mod slug;
pub mod view;
pub mod watcher;
pub mod wikilink;

pub use entity_type::EntityType;
pub use settings::{Settings, Theme, load_settings, save_settings};
pub use slug::{generate_id, generate_slug};
pub use wikilink::WikiLink;

/// Crate-level error type. Wraps vaultdb-core errors plus eduport-
/// specific failure modes (schema validation, FTS5 reconcile, etc.).
#[derive(Debug, thiserror::Error)]
pub enum EduportError {
    #[error(transparent)]
    Vaultdb(#[from] vaultdb_core::VaultdbError),

    #[error("schema error: {0}")]
    Schema(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, EduportError>;

/// Re-export the foundational vaultdb-core types so eduport-tauri and
/// future eduport-cli consumers don't need a second dependency.
pub use vaultdb_core::{
    DeleteBuilder, Direction, Expr, GraphScope, LinkGraph, MoveBuilder, MutationReport, Predicate,
    Query, Record, RenameBuilder, SortKey, UpdateBuilder, Value, Vault, WriteOptions,
};
