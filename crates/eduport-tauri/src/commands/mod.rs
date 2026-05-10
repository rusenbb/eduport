//! Tauri command handlers backed by `eduport-core`.
//!
//! Each submodule mirrors one slice of the Python sidecar's HTTP API
//! and exposes synchronous `#[tauri::command]` handlers. The handlers
//! are deliberately thin: deserialise inputs → call into eduport-core
//! → serialise outputs.
//!
//! ## Naming
//!
//! Command names are prefixed `core_<noun>_<verb>` (e.g.
//! `core_entity_list`) so they don't collide with the existing
//! sidecar-bootstrap commands in `lib.rs` while the two transports
//! coexist (Phases 9–10). Phase 11 deletes the sidecar; the prefix
//! becomes a redundant historical marker but renaming would break
//! Phase 10's frontend swap mid-flight.
//!
//! ## Error model
//!
//! Tauri serialises errors via `Serialize`. We use a single
//! [`CommandError`] type that flattens domain errors into a string
//! message + a stable `code` so the frontend can branch on the
//! error class without parsing the message. This matches what the
//! Python sidecar emitted as JSON via FastAPI's exception
//! handlers.

pub mod checkbox;
pub mod eml;
pub mod entity;
pub mod properties;
pub mod schema;
pub mod search;
pub mod settings;
pub mod status;
pub mod trash;
pub mod view;

use serde::Serialize;

/// Stable error envelope returned to the frontend. The `code` is
/// the discriminator; the `message` is human-readable. Keeping
/// these stable across versions is part of the API contract — if
/// you add a code, document it; if you remove one, that's a
/// breaking change.
#[derive(Debug, Serialize, thiserror::Error)]
#[error("{code}: {message}")]
pub struct CommandError {
    pub code: &'static str,
    pub message: String,
}

impl CommandError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("invalid", message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("not_found", message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new("conflict", message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("internal", message)
    }

    pub fn not_initialised() -> Self {
        Self::new(
            "not_initialised",
            "eduport-core state not yet initialised (settings missing or boot failed)",
        )
    }
}

impl From<eduport_core::EduportError> for CommandError {
    fn from(e: eduport_core::EduportError) -> Self {
        Self::internal(e.to_string())
    }
}

impl From<eduport_core::index::IndexError> for CommandError {
    fn from(e: eduport_core::index::IndexError) -> Self {
        Self::internal(e.to_string())
    }
}

impl From<eduport_core::entity::EntityStoreError> for CommandError {
    fn from(e: eduport_core::entity::EntityStoreError) -> Self {
        match e {
            eduport_core::entity::EntityStoreError::NotFound { .. } => {
                Self::not_found(e.to_string())
            }
            other => Self::internal(other.to_string()),
        }
    }
}

impl From<std::io::Error> for CommandError {
    fn from(e: std::io::Error) -> Self {
        Self::internal(format!("io error: {e}"))
    }
}

/// Borrow the live [`crate::core_state::EduportState`] from the
/// Tauri-managed handle. Returns a friendly "not initialised" error
/// if the user hasn't completed first-run setup yet — every command
/// goes through this so the error surface is uniform.
pub fn require_state(
    handle: &crate::core_state::EduportStateHandle,
) -> Result<std::sync::Arc<crate::core_state::EduportState>, CommandError> {
    handle
        .lock()
        .map_err(|_| CommandError::internal("eduport state mutex poisoned"))?
        .as_ref()
        .cloned()
        .ok_or_else(CommandError::not_initialised)
}
