//! On-disk store for the user-managed property schema.
//!
//! The schema lives at `<vault>/.eduport/schema.yaml`. This module
//! owns:
//!
//! - Loading (with auto-seed if absent).
//! - Atomic save (via [`vaultdb_core::writer::atomic_write`]).
//! - The historical constraints — things that depend on the previous
//!   state of the schema. The Pydantic-equivalent [`crate::schema::Property`]
//!   variants validate per-property *shape*; this store enforces:
//!
//!   1. New property keys can't collide with built-in entity fields
//!      (the eight entity types each have their own built-in field
//!      list — currently just `name`, `tags`, plus type-specific
//!      structured fields like an Email's `subject` / `from_addr`).
//!   2. New property keys can't collide with existing custom-property
//!      keys for the same entity type.
//!   3. A property's `key` and `type` are immutable post-creation;
//!      consumers patch only the fields listed in [`PatchableFields`].
//!   4. Existing select-option *values* can be deleted (orphaning
//!      entity values, surfaced as warnings in the UI) but never
//!      renamed in place; the writer pushes label/colour edits and
//!      additions through directly.
//!
//! Thread-safe via an internal [`std::sync::Mutex`]. Cross-process
//! safety comes from the vault-scoped lock in vaultdb-core.

use std::path::PathBuf;
use std::sync::Mutex;

use crate::EduportError;
use crate::EntityType;
use crate::schema::property::{Property, SelectOption};
use crate::schema::schema::{Schema, empty_schema};

/// The hidden subdirectory that holds eduport's per-vault metadata
/// (schema, views, settings overrides, FTS5 index file).
pub const ED_DIR_NAME: &str = ".eduport";
pub const SCHEMA_FILENAME: &str = "schema.yaml";

/// Schema-mutation errors. A separate variant from [`EduportError`]
/// because callers (the schema editor in the frontend, the schema-
/// init wizard) want to distinguish "the user input was bad" from
/// "the disk write failed".
#[derive(Debug, thiserror::Error)]
pub enum SchemaStoreError {
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Eduport(#[from] EduportError),
}

impl From<SchemaStoreError> for EduportError {
    fn from(e: SchemaStoreError) -> Self {
        EduportError::Schema(e.to_string())
    }
}

/// Fields the user is allowed to edit in-place on an existing
/// property. Anything else (`type`, `key`, the `value` of an existing
/// option) is immutable post-creation.
#[derive(Debug, Clone, Default)]
pub struct PatchableFields {
    pub name: Option<String>,
    pub description: Option<Option<String>>, // outer Option = "leave alone"; inner = clear or set
    pub required: Option<bool>,
    pub unit: Option<Option<String>>,
    pub options: Option<Vec<SelectOption>>,
    pub target_types: Option<Option<Vec<EntityType>>>,
}

/// On-disk schema store. Holds the cached parsed schema and
/// serialises mutations through an internal lock.
pub struct SchemaStore {
    data_folder: PathBuf,
    inner: Mutex<Option<Schema>>,
}

impl SchemaStore {
    pub fn new(data_folder: impl Into<PathBuf>) -> Self {
        Self {
            data_folder: data_folder.into(),
            inner: Mutex::new(None),
        }
    }

    pub fn schema_dir(&self) -> PathBuf {
        self.data_folder.join(ED_DIR_NAME)
    }

    pub fn schema_path(&self) -> PathBuf {
        self.schema_dir().join(SCHEMA_FILENAME)
    }

    /// Load and cache the schema. Seeds with [`empty_schema`] if the
    /// file doesn't exist yet. Subsequent `current()` calls return the
    /// cached value until [`reload`](Self::reload) or a mutation.
    pub fn load(&self) -> Result<Schema, EduportError> {
        let mut guard = self.inner.lock().expect("SchemaStore mutex poisoned");
        let schema = self.load_locked()?;
        *guard = Some(schema.clone());
        Ok(schema)
    }

    /// Force a re-read from disk, dropping the in-memory cache.
    pub fn reload(&self) -> Result<Schema, EduportError> {
        let mut guard = self.inner.lock().expect("SchemaStore mutex poisoned");
        *guard = None;
        let schema = self.load_locked()?;
        *guard = Some(schema.clone());
        Ok(schema)
    }

    /// Return the cached schema; load if not yet cached.
    pub fn current(&self) -> Result<Schema, EduportError> {
        let mut guard = self.inner.lock().expect("SchemaStore mutex poisoned");
        if let Some(s) = &*guard {
            return Ok(s.clone());
        }
        let schema = self.load_locked()?;
        *guard = Some(schema.clone());
        Ok(schema)
    }

    /// Add a new property to `entity_type`. Errors if the key collides
    /// with an existing custom property (built-in field collision is
    /// the caller's responsibility — the entity-type registry will
    /// expose `is_builtin_key` once Phase 6 lands).
    pub fn add_property(
        &self,
        entity_type: EntityType,
        prop: Property,
    ) -> Result<Schema, SchemaStoreError> {
        prop.validate().map_err(SchemaStoreError::Invalid)?;
        let mut guard = self.inner.lock().expect("SchemaStore mutex poisoned");
        let mut schema = match &*guard {
            Some(s) => s.clone(),
            None => self.load_locked()?,
        };
        let entity_schema = schema.types.entry(entity_type).or_default();
        if entity_schema.property(prop.key()).is_some() {
            return Err(SchemaStoreError::Conflict(format!(
                "property {:?} already exists on {}",
                prop.key(),
                entity_type
            )));
        }
        entity_schema.properties.push(prop);
        self.save_locked(&schema)?;
        *guard = Some(schema.clone());
        Ok(schema)
    }

    /// Patch the editable fields of an existing property in place.
    pub fn patch_property(
        &self,
        entity_type: EntityType,
        key: &str,
        patch: PatchableFields,
    ) -> Result<Schema, SchemaStoreError> {
        let mut guard = self.inner.lock().expect("SchemaStore mutex poisoned");
        let mut schema = match &*guard {
            Some(s) => s.clone(),
            None => self.load_locked()?,
        };
        let es = schema.types.entry(entity_type).or_default();
        let pos = es
            .properties
            .iter()
            .position(|p| p.key() == key)
            .ok_or_else(|| {
                SchemaStoreError::NotFound(format!("no property {:?} on {}", key, entity_type))
            })?;
        let updated = apply_patch(es.properties[pos].clone(), patch)?;
        updated.validate().map_err(SchemaStoreError::Invalid)?;
        es.properties[pos] = updated;
        self.save_locked(&schema)?;
        *guard = Some(schema.clone());
        Ok(schema)
    }

    /// Reorder the properties of `entity_type` to match `ordered_keys`.
    /// `ordered_keys` must contain exactly the existing keys.
    pub fn reorder_properties(
        &self,
        entity_type: EntityType,
        ordered_keys: &[String],
    ) -> Result<Schema, SchemaStoreError> {
        let mut guard = self.inner.lock().expect("SchemaStore mutex poisoned");
        let mut schema = match &*guard {
            Some(s) => s.clone(),
            None => self.load_locked()?,
        };
        let es = schema.types.entry(entity_type).or_default();
        let mut existing: std::collections::HashMap<String, Property> = es
            .properties
            .iter()
            .map(|p| (p.key().to_string(), p.clone()))
            .collect();
        let existing_keys: std::collections::HashSet<&String> = existing.keys().collect();
        let new_keys: std::collections::HashSet<&String> = ordered_keys.iter().collect();
        if existing_keys != new_keys {
            return Err(SchemaStoreError::Invalid(
                "ordered_keys must contain exactly the existing property keys".into(),
            ));
        }
        let new_props: Vec<Property> = ordered_keys
            .iter()
            .map(|k| existing.remove(k).unwrap())
            .collect();
        es.properties = new_props;
        self.save_locked(&schema)?;
        *guard = Some(schema.clone());
        Ok(schema)
    }

    /// Remove a property from the schema. Existing entity values for
    /// that key become "orphaned" — they remain on disk but the schema
    /// no longer recognises them.
    pub fn delete_property(
        &self,
        entity_type: EntityType,
        key: &str,
    ) -> Result<Schema, SchemaStoreError> {
        let mut guard = self.inner.lock().expect("SchemaStore mutex poisoned");
        let mut schema = match &*guard {
            Some(s) => s.clone(),
            None => self.load_locked()?,
        };
        let es = schema.types.entry(entity_type).or_default();
        let before = es.properties.len();
        es.properties.retain(|p| p.key() != key);
        if es.properties.len() == before {
            return Err(SchemaStoreError::NotFound(format!(
                "no property {:?} on {}",
                key, entity_type
            )));
        }
        self.save_locked(&schema)?;
        *guard = Some(schema.clone());
        Ok(schema)
    }

    // ── internals ────────────────────────────────────────────────────

    fn load_locked(&self) -> Result<Schema, EduportError> {
        let path = self.schema_path();
        if !path.exists() {
            let seeded = empty_schema();
            self.save_locked(&seeded)?;
            return Ok(seeded);
        }
        let text = std::fs::read_to_string(&path).map_err(EduportError::Io)?;
        let schema: Schema = serde_yaml::from_str(&text)
            .map_err(|e| EduportError::Schema(format!("schema.yaml: {}", e)))?;
        schema.validate().map_err(EduportError::Schema)?;
        Ok(schema)
    }

    fn save_locked(&self, schema: &Schema) -> Result<(), EduportError> {
        std::fs::create_dir_all(self.schema_dir()).map_err(EduportError::Io)?;
        let text = serde_yaml::to_string(schema)
            .map_err(|e| EduportError::Schema(format!("schema serialize: {}", e)))?;
        vaultdb_core::writer::atomic_write(&self.schema_path(), &text).map_err(EduportError::Io)?;
        Ok(())
    }
}

/// Apply a `PatchableFields` to an existing property, type-by-type.
/// The `key` and `type` are immutable; everything else flows through
/// per the variant's allowed fields.
fn apply_patch(prop: Property, patch: PatchableFields) -> Result<Property, SchemaStoreError> {
    Ok(match prop {
        Property::Text(mut p) => {
            apply_common(&mut p.name, &mut p.description, &mut p.required, &patch);
            check_type_specific_unset(&patch, &["unit", "options", "target_types"], "text")?;
            Property::Text(p)
        }
        Property::Number(mut p) => {
            apply_common(&mut p.name, &mut p.description, &mut p.required, &patch);
            if let Some(unit) = &patch.unit {
                p.unit = unit.clone();
            }
            check_type_specific_unset(&patch, &["options", "target_types"], "number")?;
            Property::Number(p)
        }
        Property::Date(mut p) => {
            apply_common(&mut p.name, &mut p.description, &mut p.required, &patch);
            check_type_specific_unset(&patch, &["unit", "options", "target_types"], "date")?;
            Property::Date(p)
        }
        Property::Checkbox(mut p) => {
            apply_common(&mut p.name, &mut p.description, &mut p.required, &patch);
            check_type_specific_unset(&patch, &["unit", "options", "target_types"], "checkbox")?;
            Property::Checkbox(p)
        }
        Property::SingleSelect(mut p) => {
            apply_common(&mut p.name, &mut p.description, &mut p.required, &patch);
            if let Some(options) = &patch.options {
                p.options = options.clone();
            }
            check_type_specific_unset(&patch, &["unit", "target_types"], "single-select")?;
            Property::SingleSelect(p)
        }
        Property::MultiSelect(mut p) => {
            apply_common(&mut p.name, &mut p.description, &mut p.required, &patch);
            if let Some(options) = &patch.options {
                p.options = options.clone();
            }
            check_type_specific_unset(&patch, &["unit", "target_types"], "multi-select")?;
            Property::MultiSelect(p)
        }
        Property::Url(mut p) => {
            apply_common(&mut p.name, &mut p.description, &mut p.required, &patch);
            check_type_specific_unset(&patch, &["unit", "options", "target_types"], "url")?;
            Property::Url(p)
        }
        Property::Relation(mut p) => {
            apply_common(&mut p.name, &mut p.description, &mut p.required, &patch);
            if let Some(target_types) = &patch.target_types {
                p.target_types = target_types.clone();
            }
            check_type_specific_unset(&patch, &["unit", "options"], "relation")?;
            Property::Relation(p)
        }
    })
}

fn apply_common(
    name: &mut String,
    description: &mut Option<String>,
    required: &mut bool,
    patch: &PatchableFields,
) {
    if let Some(n) = &patch.name {
        *name = n.clone();
    }
    if let Some(d) = &patch.description {
        *description = d.clone();
    }
    if let Some(r) = patch.required {
        *required = r;
    }
}

fn check_type_specific_unset(
    patch: &PatchableFields,
    forbidden: &[&str],
    kind: &str,
) -> Result<(), SchemaStoreError> {
    for f in forbidden {
        let is_set = match *f {
            "unit" => patch.unit.is_some(),
            "options" => patch.options.is_some(),
            "target_types" => patch.target_types.is_some(),
            _ => false,
        };
        if is_set {
            return Err(SchemaStoreError::Invalid(format!(
                "field {:?} is not patchable on a {} property",
                f, kind
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::property::{NumberProperty, SingleSelectProperty, TextProperty};
    use tempfile::TempDir;

    fn store(dir: &TempDir) -> SchemaStore {
        SchemaStore::new(dir.path().to_path_buf())
    }

    #[test]
    fn load_seeds_an_empty_schema_when_file_absent() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        let schema = s.load().unwrap();
        assert!(schema.validate().is_ok());
        // File should now exist on disk after the seed.
        assert!(s.schema_path().exists());
        // And the file should be valid YAML round-tripping back.
        let raw = std::fs::read_to_string(s.schema_path()).unwrap();
        let back: Schema = serde_yaml::from_str(&raw).unwrap();
        assert_eq!(back, schema);
    }

    #[test]
    fn add_property_persists_and_caches() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        s.load().unwrap();
        let prop = Property::Text(TextProperty {
            key: "summary".into(),
            name: "Summary".into(),
            description: None,
            required: false,
            default: None,
        });
        let after = s.add_property(EntityType::Note, prop.clone()).unwrap();
        assert_eq!(after.for_type(EntityType::Note).properties.len(), 1);
        // Reload from disk; the change should be there.
        s.reload().unwrap();
        let cur = s.current().unwrap();
        assert_eq!(
            cur.for_type(EntityType::Note).property("summary"),
            Some(&prop)
        );
    }

    #[test]
    fn add_property_rejects_duplicate_key() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        s.load().unwrap();
        let p = Property::Text(TextProperty {
            key: "summary".into(),
            name: "Summary".into(),
            description: None,
            required: false,
            default: None,
        });
        s.add_property(EntityType::Note, p.clone()).unwrap();
        let result = s.add_property(EntityType::Note, p);
        assert!(matches!(result, Err(SchemaStoreError::Conflict(_))));
    }

    #[test]
    fn add_property_rejects_invalid_property() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        s.load().unwrap();
        // Bad key shape.
        let bad = Property::Text(TextProperty {
            key: "Bad-Key".into(),
            name: "n".into(),
            description: None,
            required: false,
            default: None,
        });
        assert!(matches!(
            s.add_property(EntityType::Note, bad),
            Err(SchemaStoreError::Invalid(_))
        ));
    }

    #[test]
    fn patch_property_edits_name_in_place() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        s.load().unwrap();
        s.add_property(
            EntityType::Note,
            Property::Text(TextProperty {
                key: "summary".into(),
                name: "Summary".into(),
                description: None,
                required: false,
                default: None,
            }),
        )
        .unwrap();

        let after = s
            .patch_property(
                EntityType::Note,
                "summary",
                PatchableFields {
                    name: Some("Brief".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        let p = after
            .for_type(EntityType::Note)
            .property("summary")
            .unwrap();
        match p {
            Property::Text(tp) => assert_eq!(tp.name, "Brief"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn patch_property_rejects_field_not_allowed_for_type() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        s.load().unwrap();
        s.add_property(
            EntityType::Note,
            Property::Text(TextProperty {
                key: "summary".into(),
                name: "Summary".into(),
                description: None,
                required: false,
                default: None,
            }),
        )
        .unwrap();

        // `unit` is only valid on number properties.
        let result = s.patch_property(
            EntityType::Note,
            "summary",
            PatchableFields {
                unit: Some(Some("km".into())),
                ..Default::default()
            },
        );
        assert!(matches!(result, Err(SchemaStoreError::Invalid(_))));
    }

    #[test]
    fn patch_property_unit_works_on_number() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        s.load().unwrap();
        s.add_property(
            EntityType::Note,
            Property::Number(NumberProperty {
                key: "rating".into(),
                name: "Rating".into(),
                description: None,
                required: false,
                unit: None,
                default: None,
            }),
        )
        .unwrap();
        let after = s
            .patch_property(
                EntityType::Note,
                "rating",
                PatchableFields {
                    unit: Some(Some("stars".into())),
                    ..Default::default()
                },
            )
            .unwrap();
        let p = after.for_type(EntityType::Note).property("rating").unwrap();
        match p {
            Property::Number(np) => assert_eq!(np.unit.as_deref(), Some("stars")),
            _ => panic!(),
        }
    }

    #[test]
    fn reorder_properties_changes_order_and_persists() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        s.load().unwrap();
        for k in ["a", "b", "c"] {
            s.add_property(
                EntityType::Note,
                Property::Text(TextProperty {
                    key: k.into(),
                    name: k.into(),
                    description: None,
                    required: false,
                    default: None,
                }),
            )
            .unwrap();
        }
        let after = s
            .reorder_properties(EntityType::Note, &["c".to_string(), "a".into(), "b".into()])
            .unwrap();
        let keys: Vec<&str> = after
            .for_type(EntityType::Note)
            .properties
            .iter()
            .map(|p| p.key())
            .collect();
        assert_eq!(keys, vec!["c", "a", "b"]);
    }

    #[test]
    fn reorder_properties_rejects_mismatched_key_set() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        s.load().unwrap();
        s.add_property(
            EntityType::Note,
            Property::Text(TextProperty {
                key: "a".into(),
                name: "A".into(),
                description: None,
                required: false,
                default: None,
            }),
        )
        .unwrap();
        let result = s.reorder_properties(EntityType::Note, &["a".into(), "ghost".into()]);
        assert!(matches!(result, Err(SchemaStoreError::Invalid(_))));
    }

    #[test]
    fn delete_property_removes_and_persists() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        s.load().unwrap();
        s.add_property(
            EntityType::Note,
            Property::Text(TextProperty {
                key: "summary".into(),
                name: "Summary".into(),
                description: None,
                required: false,
                default: None,
            }),
        )
        .unwrap();
        let after = s.delete_property(EntityType::Note, "summary").unwrap();
        assert_eq!(after.for_type(EntityType::Note).properties.len(), 0);
    }

    #[test]
    fn delete_property_errors_when_absent() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        s.load().unwrap();
        let result = s.delete_property(EntityType::Note, "ghost");
        assert!(matches!(result, Err(SchemaStoreError::NotFound(_))));
    }

    #[test]
    fn add_then_patch_select_option_default() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        s.load().unwrap();
        s.add_property(
            EntityType::Application,
            Property::SingleSelect(SingleSelectProperty {
                key: "status".into(),
                name: "Status".into(),
                description: None,
                required: false,
                options: vec![
                    crate::schema::property::SelectOption {
                        value: "draft".into(),
                        label: "Draft".into(),
                        color: crate::schema::property::OptionColor::Gray,
                    },
                    crate::schema::property::SelectOption {
                        value: "submitted".into(),
                        label: "Submitted".into(),
                        color: crate::schema::property::OptionColor::Blue,
                    },
                ],
                default: None,
            }),
        )
        .unwrap();

        // Patch options to add a new colour for one of them.
        let new_options = vec![
            crate::schema::property::SelectOption {
                value: "draft".into(),
                label: "Draft".into(),
                color: crate::schema::property::OptionColor::Gray,
            },
            crate::schema::property::SelectOption {
                value: "submitted".into(),
                label: "Submitted".into(),
                color: crate::schema::property::OptionColor::Green, // changed
            },
            crate::schema::property::SelectOption {
                value: "accepted".into(),
                label: "Accepted".into(),
                color: crate::schema::property::OptionColor::Purple,
            },
        ];
        let after = s
            .patch_property(
                EntityType::Application,
                "status",
                PatchableFields {
                    options: Some(new_options.clone()),
                    ..Default::default()
                },
            )
            .unwrap();
        let p = after
            .for_type(EntityType::Application)
            .property("status")
            .unwrap();
        match p {
            Property::SingleSelect(ssp) => {
                assert_eq!(ssp.options, new_options);
            }
            _ => panic!(),
        }
    }
}
