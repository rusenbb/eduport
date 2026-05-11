//! User-managed property schema for eduport's eight entity types.
//!
//! The schema lives at `<vault>/.eduport/schema.yaml`. It declares
//! per-entity custom properties (text, number, date, checkbox,
//! single-select, multi-select, url, relation) with their type-specific
//! constraints (units, options, target types, etc.). The schema is
//! strict about its own shape (`deny_unknown_fields`) but the
//! corresponding entity records remain lenient — extra keys in
//! frontmatter that don't match any declared property are kept and
//! surfaced to the user as "orphaned values" rather than rejected.
//!
//! Three layers:
//!
//! - [`property::Property`] — the property variants and their fields.
//! - [`schema::Schema`] / [`schema::EntitySchema`] — the file shape and
//!   the per-entity-type collection of properties.
//! - [`store::SchemaStore`] — load, atomic save, and the historical
//!   constraints (no rename of existing select option values, no type
//!   change post-creation, etc.).

pub mod builtins;
pub mod property;
#[allow(clippy::module_inception)]
pub mod schema;
pub mod store;

pub use builtins::{is_builtin_key, seeded_builtins};
pub use property::{
    CheckboxProperty, DateProperty, MultiSelectProperty, NumberProperty, OptionColor, Property,
    PropertyKind, RelationProperty, SelectOption, SingleSelectProperty, TextProperty, UrlProperty,
};
pub use schema::{EntitySchema, SCHEMA_VERSION, Schema, default_schema, empty_schema};
pub use store::{PatchableFields, SchemaStore, SchemaStoreError};
