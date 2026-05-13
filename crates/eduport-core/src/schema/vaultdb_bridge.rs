//! Bridge between eduport's Notion-style [`crate::schema::Schema`] and
//! vaultdb's runtime [`vaultdb_core::schema::CollectionSchema`].
//!
//! Eduport's schema is richer than vaultdb's — it carries UI metadata
//! (colored option labels, descriptions, target-type constraints on
//! relations). Vaultdb-core only cares about validation: types,
//! enums, defaults, required. This module converts the eduport view
//! into the vaultdb view, lossily — anything purely cosmetic stays in
//! eduport-core.
//!
//! Returned `CollectionSchema`s are intended for runtime use with
//! `vaultdb_orm::Create::<T>::with_schema(...)` so that schema defaults
//! and required-field enforcement apply to eduport's typed entity
//! creates. The bridge does NOT persist anything; eduport's
//! `.eduport/schema.yaml` remains the source of truth.

use std::collections::BTreeMap;

use vaultdb_core::Value;
use vaultdb_core::schema::{CollectionSchema, FieldSchema};

use crate::EntityType;
use crate::schema::{Property, Schema};

/// Build the vaultdb [`CollectionSchema`] equivalent of an eduport
/// entity type's properties, plus the few built-in frontmatter fields
/// every eduport entity carries (`name`, `tags`).
///
/// The folder is left empty — eduport entities are tag-discriminated
/// rather than folder-bound — and the filter clause restates the
/// `eduport-type/<type>` tag check so the result is also valid as a
/// `vaultdb schema validate` collection if anyone ever serialises it.
pub fn collection_for(entity_type: EntityType, schema: &Schema) -> CollectionSchema {
    let entity = schema
        .types
        .get(&entity_type)
        .expect("Schema invariant: every EntityType has an EntitySchema entry");

    let mut fields = BTreeMap::new();
    let mut required = Vec::new();

    // Built-in frontmatter fields every entity carries.
    fields.insert(
        "name".into(),
        FieldSchema {
            field_type: "string".into(),
            enum_values: vec![],
            min: None,
            max: None,
            default: None,
            default_expr: None,
        },
    );
    fields.insert(
        "tags".into(),
        FieldSchema {
            field_type: "list".into(),
            enum_values: vec![],
            min: None,
            max: None,
            default: None,
            default_expr: None,
        },
    );
    required.push("name".to_string());
    required.push("tags".to_string());

    // User-declared (and built-in via builtins module) properties.
    for prop in &entity.properties {
        let key = prop.key().to_string();
        let field = property_to_field_schema(prop);
        fields.insert(key.clone(), field);
        if prop_is_required(prop) {
            required.push(key);
        }
    }

    CollectionSchema {
        description: Some(format!(
            "eduport {} entities (derived from .eduport/schema.yaml)",
            entity_type
        )),
        folder: String::new(), // tag-discriminated, not folder-bound
        filter: vec![format!("tags contains eduport-type/{}", entity_type)],
        required,
        fields,
    }
}

fn prop_is_required(p: &Property) -> bool {
    match p {
        Property::Text(p) => p.required,
        Property::Number(p) => p.required,
        Property::Date(p) => p.required,
        Property::Checkbox(p) => p.required,
        Property::SingleSelect(p) => p.required,
        Property::MultiSelect(p) => p.required,
        Property::Url(p) => p.required,
        Property::Relation(p) => p.required,
    }
}

fn property_to_field_schema(p: &Property) -> FieldSchema {
    match p {
        Property::Text(p) => FieldSchema {
            field_type: "string".into(),
            enum_values: vec![],
            min: None,
            max: None,
            default: p.default.clone().map(Value::String),
            default_expr: None,
        },
        Property::Number(p) => FieldSchema {
            field_type: "number".into(),
            enum_values: vec![],
            min: None,
            max: None,
            default: p.default.map(Value::Float),
            default_expr: None,
        },
        Property::Date(p) => FieldSchema {
            field_type: "date".into(),
            enum_values: vec![],
            min: None,
            max: None,
            default: p.default.clone().map(Value::String),
            default_expr: None,
        },
        Property::Checkbox(p) => FieldSchema {
            field_type: "bool".into(),
            enum_values: vec![],
            min: None,
            max: None,
            default: p.default.map(Value::Bool),
            default_expr: None,
        },
        Property::SingleSelect(p) => FieldSchema {
            field_type: "string".into(),
            // Eduport options have value/label/color; vaultdb only
            // cares about the value side. Labels and colors remain in
            // the eduport Schema for the UI.
            enum_values: p
                .options
                .iter()
                .map(|o| Value::String(o.value.clone()))
                .collect(),
            min: None,
            max: None,
            default: p.default.clone().map(Value::String),
            default_expr: None,
        },
        Property::MultiSelect(p) => FieldSchema {
            field_type: "list".into(),
            // vaultdb doesn't yet model typed lists; storing the
            // option values on a `list` field is informational only.
            enum_values: p
                .options
                .iter()
                .map(|o| Value::String(o.value.clone()))
                .collect(),
            min: None,
            max: None,
            default: p
                .default
                .as_ref()
                .map(|defs| Value::List(defs.iter().cloned().map(Value::String).collect())),
            default_expr: None,
        },
        Property::Url(p) => FieldSchema {
            field_type: "url".into(),
            enum_values: vec![],
            min: None,
            max: None,
            default: p.default.clone().map(Value::String),
            default_expr: None,
        },
        Property::Relation(p) => FieldSchema {
            // Eduport relations carry a `target_types` constraint that
            // vaultdb's `wikilink` type doesn't model. We map both to
            // `wikilink` for runtime validation; the target-type check
            // stays an eduport-side concern.
            field_type: "wikilink".into(),
            enum_values: vec![],
            min: None,
            max: None,
            default: p.default.clone().map(Value::String),
            default_expr: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{
        DateProperty, NumberProperty, Property, SelectOption, SingleSelectProperty, TextProperty,
        UrlProperty,
    };

    fn empty_schema_with(entity: EntityType, props: Vec<Property>) -> Schema {
        let mut s = crate::schema::schema::empty_schema();
        s.types.get_mut(&entity).unwrap().properties = props;
        s
    }

    #[test]
    fn built_in_fields_always_present() {
        let schema = crate::schema::schema::empty_schema();
        let c = collection_for(EntityType::Person, &schema);
        assert!(c.fields.contains_key("name"));
        assert!(c.fields.contains_key("tags"));
        assert!(c.required.contains(&"name".to_string()));
        assert!(c.required.contains(&"tags".to_string()));
    }

    #[test]
    fn discriminator_filter_uses_tag() {
        let schema = crate::schema::schema::empty_schema();
        let c = collection_for(EntityType::Person, &schema);
        assert_eq!(c.filter, vec!["tags contains eduport-type/person"]);
    }

    #[test]
    fn text_property_maps_to_string_with_default() {
        let schema = empty_schema_with(
            EntityType::Person,
            vec![Property::Text(TextProperty {
                key: "role".into(),
                name: "Role".into(),
                description: None,
                required: true,
                is_builtin: false,
                default: Some("associate-professor".into()),
            })],
        );
        let c = collection_for(EntityType::Person, &schema);
        let f = c.fields.get("role").unwrap();
        assert_eq!(f.field_type, "string");
        assert_eq!(f.default, Some(Value::String("associate-professor".into())));
        assert!(c.required.contains(&"role".to_string()));
    }

    #[test]
    fn date_property_maps_to_date_type() {
        let schema = empty_schema_with(
            EntityType::Application,
            vec![Property::Date(DateProperty {
                key: "deadline".into(),
                name: "Deadline".into(),
                description: None,
                required: false,
                is_builtin: false,
                default: None,
            })],
        );
        let c = collection_for(EntityType::Application, &schema);
        assert_eq!(c.fields.get("deadline").unwrap().field_type, "date");
    }

    #[test]
    fn url_property_maps_to_url_type() {
        let schema = empty_schema_with(
            EntityType::University,
            vec![Property::Url(UrlProperty {
                key: "homepage".into(),
                name: "Homepage".into(),
                description: None,
                required: false,
                is_builtin: false,
                default: None,
            })],
        );
        let c = collection_for(EntityType::University, &schema);
        assert_eq!(c.fields.get("homepage").unwrap().field_type, "url");
    }

    #[test]
    fn single_select_maps_to_string_with_enum() {
        let schema = empty_schema_with(
            EntityType::University,
            vec![Property::SingleSelect(SingleSelectProperty {
                key: "tier".into(),
                name: "Tier".into(),
                description: None,
                required: false,
                is_builtin: false,
                options: vec![
                    SelectOption {
                        value: "high".into(),
                        label: "High".into(),
                        color: Default::default(),
                    },
                    SelectOption {
                        value: "low".into(),
                        label: "Low".into(),
                        color: Default::default(),
                    },
                ],
                default: None,
            })],
        );
        let c = collection_for(EntityType::University, &schema);
        let f = c.fields.get("tier").unwrap();
        assert_eq!(f.field_type, "string");
        assert!(f.enum_values.contains(&Value::String("high".into())));
        assert!(f.enum_values.contains(&Value::String("low".into())));
    }

    #[test]
    fn number_property_default_is_float() {
        let schema = empty_schema_with(
            EntityType::Program,
            vec![Property::Number(NumberProperty {
                key: "tuition".into(),
                name: "Tuition".into(),
                description: None,
                required: false,
                is_builtin: false,
                unit: Some("USD".into()),
                default: Some(50_000.0),
            })],
        );
        let c = collection_for(EntityType::Program, &schema);
        let f = c.fields.get("tuition").unwrap();
        assert_eq!(f.field_type, "number");
        assert_eq!(f.default, Some(Value::Float(50_000.0)));
    }

    #[test]
    fn passes_vaultdb_default_validation() {
        // The output of `collection_for` should round-trip through
        // vaultdb's `validate_schema_defaults` without errors —
        // otherwise the bridge is producing invalid schemas.
        let schema = empty_schema_with(
            EntityType::Person,
            vec![
                Property::Text(TextProperty {
                    key: "role".into(),
                    name: "Role".into(),
                    description: None,
                    required: false,
                    is_builtin: false,
                    default: Some("associate-professor".into()),
                }),
                Property::Url(UrlProperty {
                    key: "homepage".into(),
                    name: "Homepage".into(),
                    description: None,
                    required: false,
                    is_builtin: false,
                    default: Some("https://example.com".into()),
                }),
            ],
        );
        let c = collection_for(EntityType::Person, &schema);
        let mut vs = vaultdb_core::schema::VaultSchema {
            collections: BTreeMap::new(),
        };
        vs.collections.insert("person".into(), c);
        vaultdb_core::schema::validate_schema_defaults(&vs)
            .expect("bridge output must be valid vaultdb schema");
    }
}
