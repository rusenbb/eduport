//! Top-level schema types: [`EntitySchema`] (per-entity-type property
//! list) and [`Schema`] (the full file with one entry per
//! [`EntityType`]).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::EntityType;
use crate::schema::property::Property;

pub const SCHEMA_VERSION: u32 = 1;

/// A single entity type's collection of user-declared properties.
/// Property keys must be unique within this collection.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields)]
pub struct EntitySchema {
    #[serde(default)]
    pub properties: Vec<Property>,
}

impl EntitySchema {
    /// Look up a property by its `key`. Returns `None` if no such
    /// property exists on this entity type.
    pub fn property(&self, key: &str) -> Option<&Property> {
        self.properties.iter().find(|p| p.key() == key)
    }

    /// Validate per-entity invariants: every property's own validate()
    /// passes AND keys are unique.
    pub fn validate(&self) -> Result<(), String> {
        let mut seen = std::collections::HashSet::new();
        for p in &self.properties {
            p.validate()?;
            if !seen.insert(p.key()) {
                return Err(format!("duplicate property key: {:?}", p.key()));
            }
        }
        Ok(())
    }
}

/// The full user-managed schema. Every [`EntityType`] must have an
/// entry — [`empty_schema`] seeds with all eight types and no
/// properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields)]
pub struct Schema {
    #[serde(default = "default_version")]
    pub version: u32,
    pub types: BTreeMap<EntityType, EntitySchema>,
}

fn default_version() -> u32 {
    SCHEMA_VERSION
}

impl Schema {
    /// Get the schema entry for `entity_type`. Panics if absent — by
    /// invariant every type has an entry; if you got a Schema through
    /// the public load_schema path, this can't happen.
    pub fn for_type(&self, entity_type: EntityType) -> &EntitySchema {
        self.types
            .get(&entity_type)
            .expect("schema is missing an entity type entry; load_schema enforces this invariant")
    }

    /// Validate the full schema: version supported, all entity types
    /// present, every per-type EntitySchema valid.
    pub fn validate(&self) -> Result<(), String> {
        if self.version != SCHEMA_VERSION {
            return Err(format!(
                "unsupported schema version {}; this build expects {}",
                self.version, SCHEMA_VERSION
            ));
        }
        for t in EntityType::ALL {
            if !self.types.contains_key(&t) {
                return Err(format!("schema missing entry for entity type {:?}", t));
            }
        }
        for es in self.types.values() {
            es.validate()?;
        }
        Ok(())
    }
}

/// Schema with no properties at all — every entity type gets an empty
/// property list. Used by tests that need to start from a clean slate.
/// Production code wants [`default_schema`] (which includes the
/// system-seeded built-ins).
pub fn empty_schema() -> Schema {
    let types = EntityType::ALL
        .into_iter()
        .map(|t| (t, EntitySchema::default()))
        .collect();
    Schema {
        version: SCHEMA_VERSION,
        types,
    }
}

/// Schema seeded with the system-defined built-in properties for each
/// entity type. This is what fresh vaults get on first load — the
/// built-ins ship with curated option lists (countries, languages,
/// roles, etc.) and the user can extend them through the regular
/// schema editing surface, just like custom properties.
pub fn default_schema() -> Schema {
    let mut s = empty_schema();
    s.merge_in_builtins();
    s
}

impl Schema {
    /// Add any missing built-in properties to every entity type's
    /// schema entry, preserving user-edited copies of built-ins that
    /// already exist (the user may have added options or renamed the
    /// label). Run on every load so adopting a new built-in field
    /// later doesn't require a manual migration.
    pub fn merge_in_builtins(&mut self) {
        for t in EntityType::ALL {
            let entry = self.types.entry(t).or_default();
            let existing_keys: std::collections::HashSet<String> = entry
                .properties
                .iter()
                .map(|p| p.key().to_string())
                .collect();
            for builtin in crate::schema::builtins::seeded_builtins(t) {
                if !existing_keys.contains(builtin.key()) {
                    entry.properties.push(builtin);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::property::{Property, TextProperty};

    #[test]
    fn empty_schema_has_all_eight_types() {
        let s = empty_schema();
        assert_eq!(s.version, SCHEMA_VERSION);
        assert_eq!(s.types.len(), 8);
        for t in EntityType::ALL {
            assert!(s.types.contains_key(&t));
            assert!(s.for_type(t).properties.is_empty());
        }
    }

    #[test]
    fn validate_rejects_missing_entity_type_entry() {
        let mut s = empty_schema();
        s.types.remove(&EntityType::Note);
        assert!(s.validate().is_err());
    }

    #[test]
    fn validate_rejects_unsupported_version() {
        let mut s = empty_schema();
        s.version = 99;
        assert!(s.validate().is_err());
    }

    #[test]
    fn validate_rejects_duplicate_property_keys_within_a_type() {
        let mut s = empty_schema();
        let dup_key = TextProperty {
            key: "summary".into(),
            name: "Summary".into(),
            description: None,
            required: false,
            is_builtin: false,
            default: None,
        };
        s.types.get_mut(&EntityType::Note).unwrap().properties =
            vec![Property::Text(dup_key.clone()), Property::Text(dup_key)];
        assert!(s.validate().is_err());
    }

    #[test]
    fn round_trip_schema_through_yaml() {
        let mut s = empty_schema();
        s.types.get_mut(&EntityType::Note).unwrap().properties =
            vec![Property::Text(TextProperty {
                key: "summary".into(),
                name: "Summary".into(),
                description: Some("a one-liner".into()),
                required: false,
                is_builtin: false,
                default: None,
            })];
        let yaml = serde_yaml::to_string(&s).unwrap();
        let back: Schema = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(back, s);
        assert!(back.validate().is_ok());
    }

    #[test]
    fn entity_schema_property_lookup() {
        let es = EntitySchema {
            properties: vec![Property::Text(TextProperty {
                key: "summary".into(),
                name: "Summary".into(),
                description: None,
                required: false,
                is_builtin: false,
                default: None,
            })],
        };
        assert!(es.property("summary").is_some());
        assert!(es.property("missing").is_none());
    }
}
