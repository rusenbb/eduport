//! The eight entity types and their shared shape.
//!
//! Each entity type has its own struct (`University`, `Lab`, ...) that
//! carries its built-in fields directly and a `custom` map that holds
//! any extra YAML keys (i.e. user-declared schema properties). The
//! [`Entity`] enum wraps all eight for code paths that handle "any
//! entity" (lists, search, watcher events).
//!
//! Loading from YAML: [`Entity::from_yaml`] inspects the `tags` list
//! to find the `eduport-type/<value>` discriminator, then deserialises
//! into the matching variant. The Pydantic original used `extra =
//! "allow"`; we replicate that with `#[serde(flatten)]` into a
//! `BTreeMap<String, serde_json::Value>` for the custom-property tail.

pub mod resources;
pub mod store;
pub mod types;

pub use resources::{EmailResource, LinkResource};
pub use store::{EntityStore, EntityStoreError};
pub use types::{
    Application, ApplicationStatus, BaseEntityFields, Document, DocumentStatus,
    EDUPORT_TYPE_PREFIX, Email, EmailDirection, Entity, Lab, Note, Person, Program, ProgramLevel,
    University, record_eduport_type,
};
