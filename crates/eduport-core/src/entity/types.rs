//! Concrete entity types: one struct per entity type, plus the
//! [`Entity`] enum that wraps any of them.
//!
//! The shape closely mirrors the Pydantic models in
//! `sidecar/src/eduport/models/`. Built-in fields are directly typed;
//! user-declared schema properties land in the per-entity `custom`
//! map (`#[serde(flatten)]`) so the YAML round-trips cleanly.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::EntityType;
use crate::entity::resources::{EmailResource, LinkResource};
use crate::wikilink::WikiLink;

/// Tag prefix that identifies an entity's type, e.g.
/// `eduport-type/university`. Required on every entity record.
pub const EDUPORT_TYPE_PREFIX: &str = "eduport-type/";

/// Fields every entity carries. `name` is required; `tags` must
/// include exactly one `eduport-type/<value>` discriminator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct BaseEntityFields {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl BaseEntityFields {
    /// Read the entity's discriminator from its tag list. Returns the
    /// first matching `eduport-type/<value>` tag.
    pub fn entity_type_from_tags(&self) -> Result<EntityType, String> {
        for tag in &self.tags {
            if let Some(rest) = tag.strip_prefix(EDUPORT_TYPE_PREFIX) {
                return rest
                    .parse::<EntityType>()
                    .map_err(|e| format!("invalid entity type tag {:?}: {}", tag, e));
            }
        }
        Err(format!(
            "entity must have an {}* tag (got tags: {:?})",
            EDUPORT_TYPE_PREFIX, self.tags
        ))
    }

    /// Tags excluding eduport-managed discriminator tags
    /// (`eduport-type/*`, `eduport-doctype/*`).
    pub fn user_tags(&self) -> Vec<&str> {
        self.tags
            .iter()
            .map(String::as_str)
            .filter(|t| !t.starts_with(EDUPORT_TYPE_PREFIX) && !t.starts_with("eduport-doctype/"))
            .collect()
    }
}

// ─── University ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, vaultdb_orm::Note)]
#[note(
    discriminator = "tags contains eduport-type/university",
    collection = "university"
)]
pub struct University {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub country: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<LinkResource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub emails: Vec<EmailResource>,
    #[serde(flatten)]
    pub custom: BTreeMap<String, serde_json::Value>,
}

// ─── Lab ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, vaultdb_orm::Note)]
#[note(discriminator = "tags contains eduport-type/lab", collection = "lab")]
pub struct Lab {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub university: Option<WikiLink>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<LinkResource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub emails: Vec<EmailResource>,
    #[serde(flatten)]
    pub custom: BTreeMap<String, serde_json::Value>,
}

// ─── Person ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, vaultdb_orm::Note)]
#[note(
    discriminator = "tags contains eduport-type/person",
    collection = "person"
)]
pub struct Person {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub university: Option<WikiLink>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labs: Vec<WikiLink>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<LinkResource>,
    #[serde(flatten)]
    pub custom: BTreeMap<String, serde_json::Value>,
}

// ─── Program ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum ProgramLevel {
    Undergrad,
    Masters,
    Phd,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, vaultdb_orm::Note)]
#[note(
    discriminator = "tags contains eduport-type/program",
    collection = "program"
)]
pub struct Program {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<ProgramLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub department: Option<String>,
    /// One or more languages of instruction. Multi-select to handle
    /// bilingual programs (e.g. English + German). Stored as
    /// kebab-case option values (`english`, `german`) — see the
    /// `language` built-in in [`crate::schema::builtins`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub language: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    /// ISO date string `YYYY-MM-DD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    /// Tuition amount as a number (currency is left to user
    /// convention — eduport doesn't carry a currency type yet).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tuition: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub university: Option<WikiLink>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub people: Vec<WikiLink>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<LinkResource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub emails: Vec<EmailResource>,
    #[serde(flatten)]
    pub custom: BTreeMap<String, serde_json::Value>,
}

// ─── Application ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum ApplicationStatus {
    #[default]
    Planning,
    Drafting,
    Submitted,
    DecisionPending,
    Accepted,
    Rejected,
    Withdrawn,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, vaultdb_orm::Note)]
#[note(
    discriminator = "tags contains eduport-type/application",
    collection = "application"
)]
pub struct Application {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub program: WikiLink,
    #[serde(default)]
    pub status: ApplicationStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal_deadline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submitted_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_at: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub documents: Vec<WikiLink>,
    #[serde(flatten)]
    pub custom: BTreeMap<String, serde_json::Value>,
}

// ─── Document ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum DocumentStatus {
    Requested,
    Drafting,
    Received,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, vaultdb_orm::Note)]
#[note(
    discriminator = "tags contains eduport-type/document",
    collection = "document"
)]
pub struct Document {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub title: String,
    /// ISO date string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// File path relative to data folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<DocumentStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommender: Option<WikiLink>,
    #[serde(flatten)]
    pub custom: BTreeMap<String, serde_json::Value>,
}

impl Document {
    /// Apply the same default-status rule the Pydantic original
    /// applied: if `status` is None, infer from `file` (Received if
    /// the file exists, Drafting otherwise).
    pub fn normalize_status(&mut self) {
        if self.status.is_none() {
            self.status = Some(if self.file.is_some() {
                DocumentStatus::Received
            } else {
                DocumentStatus::Drafting
            });
        }
    }
}

// ─── Email ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum EmailDirection {
    Inbound,
    Outbound,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, vaultdb_orm::Note)]
#[note(
    discriminator = "tags contains eduport-type/email",
    collection = "email"
)]
pub struct Email {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub direction: EmailDirection,
    /// ISO date string.
    pub date: String,
    pub subject: String,
    /// Raw `From:` address. Serialised as `from` (matches Pydantic
    /// `Field(alias="from")` plus YAML's word-collision-with-Rust-
    /// keyword problem).
    #[serde(rename = "from")]
    pub from: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub to: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cc: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bcc: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_program: Option<WikiLink>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_application: Option<WikiLink>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_people: Vec<WikiLink>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<WikiLink>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<WikiLink>,
    #[serde(flatten)]
    pub custom: BTreeMap<String, serde_json::Value>,
}

// ─── Note ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, vaultdb_orm::Note)]
#[note(discriminator = "tags contains eduport-type/note", collection = "note")]
pub struct Note {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(flatten)]
    pub custom: BTreeMap<String, serde_json::Value>,
}

// ─── Entity wrapper ──────────────────────────────────────────────────

/// Variant-typed wrapper over the eight entity structs. Useful for
/// "any entity" code paths (lists, search results, watcher events).
///
/// The discriminator is the `eduport-type/<value>` tag inside the
/// `tags` list — not a serde tag — so we can't use serde's
/// auto-tagged enum. [`Entity::from_yaml`] does a small two-pass
/// parse: read the tags first, then deserialise into the matching
/// variant.
#[derive(Debug, Clone, PartialEq)]
pub enum Entity {
    University(University),
    Lab(Lab),
    Person(Person),
    Program(Program),
    Application(Application),
    Document(Document),
    Email(Email),
    Note(Note),
}

impl Entity {
    /// Parse an entity from a YAML frontmatter string. Inspects the
    /// `tags` list to find the `eduport-type/<value>` discriminator
    /// then deserialises into the matching variant.
    pub fn from_yaml(yaml: &str) -> Result<Entity, String> {
        let base: BaseEntityFields =
            serde_yaml::from_str(yaml).map_err(|e| format!("entity frontmatter parse: {}", e))?;
        let entity_type = base.entity_type_from_tags()?;
        deserialize_variant(yaml, entity_type)
    }

    /// Build an entity from an already-parsed vaultdb [`Record`]. This
    /// is the canonical Record → Entity path: it reuses vaultdb's
    /// parser output (no YAML reserialise round-trip) and dispatches
    /// through `vaultdb_orm::Note::from_record` per variant, so the
    /// custom-property tail lands in `BTreeMap<String,
    /// serde_json::Value>` without going through `serde_yaml::Value`.
    pub fn from_record(
        record: &vaultdb_core::Record,
        vault_root: &std::path::Path,
    ) -> Result<Entity, String> {
        use vaultdb_orm::Note as _;

        let tags = match record.fields.get("tags") {
            Some(vaultdb_core::Value::List(items)) => items,
            _ => return Err("record is missing required `tags` list".into()),
        };
        let mut entity_type: Option<EntityType> = None;
        for v in tags {
            if let vaultdb_core::Value::String(s) = v
                && let Some(rest) = s.strip_prefix(EDUPORT_TYPE_PREFIX)
            {
                entity_type = Some(
                    rest.parse::<EntityType>()
                        .map_err(|e| format!("invalid entity type tag {s:?}: {e}"))?,
                );
                break;
            }
        }
        let entity_type =
            entity_type.ok_or_else(|| format!("record has no {EDUPORT_TYPE_PREFIX}* tag"))?;

        fn map_err(e: vaultdb_orm::OrmError) -> String {
            e.to_string()
        }
        Ok(match entity_type {
            EntityType::University => {
                Entity::University(University::from_record(record, vault_root).map_err(map_err)?)
            }
            EntityType::Lab => Entity::Lab(Lab::from_record(record, vault_root).map_err(map_err)?),
            EntityType::Person => {
                Entity::Person(Person::from_record(record, vault_root).map_err(map_err)?)
            }
            EntityType::Program => {
                Entity::Program(Program::from_record(record, vault_root).map_err(map_err)?)
            }
            EntityType::Application => {
                Entity::Application(Application::from_record(record, vault_root).map_err(map_err)?)
            }
            EntityType::Document => {
                Entity::Document(Document::from_record(record, vault_root).map_err(map_err)?)
            }
            EntityType::Email => {
                Entity::Email(Email::from_record(record, vault_root).map_err(map_err)?)
            }
            EntityType::Note => {
                Entity::Note(Note::from_record(record, vault_root).map_err(map_err)?)
            }
        })
    }

    pub fn entity_type(&self) -> EntityType {
        match self {
            Entity::University(_) => EntityType::University,
            Entity::Lab(_) => EntityType::Lab,
            Entity::Person(_) => EntityType::Person,
            Entity::Program(_) => EntityType::Program,
            Entity::Application(_) => EntityType::Application,
            Entity::Document(_) => EntityType::Document,
            Entity::Email(_) => EntityType::Email,
            Entity::Note(_) => EntityType::Note,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Entity::University(e) => &e.name,
            Entity::Lab(e) => &e.name,
            Entity::Person(e) => &e.name,
            Entity::Program(e) => &e.name,
            Entity::Application(e) => &e.name,
            Entity::Document(e) => &e.name,
            Entity::Email(e) => &e.name,
            Entity::Note(e) => &e.name,
        }
    }

    pub fn tags(&self) -> &[String] {
        match self {
            Entity::University(e) => &e.tags,
            Entity::Lab(e) => &e.tags,
            Entity::Person(e) => &e.tags,
            Entity::Program(e) => &e.tags,
            Entity::Application(e) => &e.tags,
            Entity::Document(e) => &e.tags,
            Entity::Email(e) => &e.tags,
            Entity::Note(e) => &e.tags,
        }
    }

    /// User-defined custom fields — the `#[serde(flatten)]` map on each
    /// variant. The indexer uses this to populate the `properties`
    /// table and the FTS5 `custom_text` column. Built-in fields stay
    /// on the typed structs and are not surfaced here.
    pub fn custom(&self) -> &std::collections::BTreeMap<String, serde_json::Value> {
        match self {
            Entity::University(e) => &e.custom,
            Entity::Lab(e) => &e.custom,
            Entity::Person(e) => &e.custom,
            Entity::Program(e) => &e.custom,
            Entity::Application(e) => &e.custom,
            Entity::Document(e) => &e.custom,
            Entity::Email(e) => &e.custom,
            Entity::Note(e) => &e.custom,
        }
    }

    /// Serialise to a YAML frontmatter block. Round-trips with
    /// [`Self::from_yaml`].
    pub fn to_yaml(&self) -> Result<String, String> {
        match self {
            Entity::University(e) => serde_yaml::to_string(e),
            Entity::Lab(e) => serde_yaml::to_string(e),
            Entity::Person(e) => serde_yaml::to_string(e),
            Entity::Program(e) => serde_yaml::to_string(e),
            Entity::Application(e) => serde_yaml::to_string(e),
            Entity::Document(e) => serde_yaml::to_string(e),
            Entity::Email(e) => serde_yaml::to_string(e),
            Entity::Note(e) => serde_yaml::to_string(e),
        }
        .map_err(|e| format!("entity serialize: {}", e))
    }
}

/// Read a record's `eduport-type/<value>` tag and resolve it to an
/// [`EntityType`]. Returns `None` when the tag list is missing, no
/// `eduport-type/*` tag is present, or the value doesn't map to a
/// known type. Cheap projection — does not deserialise the rest of
/// the record's frontmatter into a typed Entity.
pub fn record_eduport_type(record: &vaultdb_core::Record) -> Option<EntityType> {
    let tags = match record.fields.get("tags")? {
        vaultdb_core::Value::List(items) => items,
        _ => return None,
    };
    for v in tags {
        if let vaultdb_core::Value::String(s) = v
            && let Some(rest) = s.strip_prefix(EDUPORT_TYPE_PREFIX)
            && let Ok(t) = rest.parse::<EntityType>()
        {
            return Some(t);
        }
    }
    None
}

fn deserialize_variant(yaml: &str, entity_type: EntityType) -> Result<Entity, String> {
    fn parse<T: for<'de> Deserialize<'de>>(yaml: &str) -> Result<T, String> {
        serde_yaml::from_str(yaml).map_err(|e| format!("entity parse: {}", e))
    }
    Ok(match entity_type {
        EntityType::University => Entity::University(parse(yaml)?),
        EntityType::Lab => Entity::Lab(parse(yaml)?),
        EntityType::Person => Entity::Person(parse(yaml)?),
        EntityType::Program => Entity::Program(parse(yaml)?),
        EntityType::Application => Entity::Application(parse(yaml)?),
        EntityType::Document => Entity::Document(parse(yaml)?),
        EntityType::Email => Entity::Email(parse(yaml)?),
        EntityType::Note => Entity::Note(parse(yaml)?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn university_round_trips_through_yaml() {
        let u = University {
            name: "Stanford".into(),
            tags: vec!["eduport-type/university".into()],
            country: "USA".into(),
            city: Some("Stanford, CA".into()),
            website: Some("https://stanford.edu".into()),
            links: vec![LinkResource {
                label: "Admissions".into(),
                url: "https://stanford.edu/admissions".into(),
            }],
            emails: vec![],
            custom: BTreeMap::new(),
        };
        let yaml = serde_yaml::to_string(&u).unwrap();
        let back: University = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(back, u);
    }

    #[test]
    fn entity_from_yaml_picks_the_right_variant() {
        let yaml = r#"
name: Stanford
tags:
  - eduport-type/university
country: USA
"#;
        let e = Entity::from_yaml(yaml).unwrap();
        assert_eq!(e.entity_type(), EntityType::University);
        assert_eq!(e.name(), "Stanford");
    }

    #[test]
    fn entity_from_yaml_handles_application_with_required_program() {
        let yaml = r#"
name: stanford-cs-phd-2026
tags:
  - eduport-type/application
program: "[[Stanford CS PhD 2026]]"
status: drafting
"#;
        let e = Entity::from_yaml(yaml).unwrap();
        match e {
            Entity::Application(a) => {
                assert_eq!(a.program.target, "Stanford CS PhD 2026");
                assert_eq!(a.status, ApplicationStatus::Drafting);
            }
            other => panic!("expected Application, got {:?}", other),
        }
    }

    #[test]
    fn entity_from_yaml_errors_when_type_tag_missing() {
        let yaml = r#"
name: floating
tags: []
"#;
        assert!(Entity::from_yaml(yaml).is_err());
    }

    #[test]
    fn entity_from_yaml_errors_when_type_tag_invalid() {
        let yaml = r#"
name: bad
tags:
  - eduport-type/martian
"#;
        assert!(Entity::from_yaml(yaml).is_err());
    }

    #[test]
    fn note_carries_only_name_tags_custom() {
        let yaml = r#"
name: My note
tags:
  - eduport-type/note
custom_field: hello
priority: 5
"#;
        let e = Entity::from_yaml(yaml).unwrap();
        match e {
            Entity::Note(n) => {
                assert_eq!(n.name, "My note");
                assert_eq!(n.custom.len(), 2);
                assert!(n.custom.contains_key("custom_field"));
                assert!(n.custom.contains_key("priority"));
            }
            _ => panic!("expected Note"),
        }
    }

    #[test]
    fn every_variant_emits_a_parseable_discriminator() {
        // The derive(Note) macro turns the `#[note(filter = "...")]`
        // string into an `Expr::parse(...).ok()` call. If a filter
        // string typoes the where-DSL, .ok() silently returns None and
        // Query<T> would scan the entire folder without any pinning.
        // This test pins down that every variant's filter parses.
        use vaultdb_orm::Note as _;
        assert!(super::University::discriminator().is_some());
        assert!(super::Lab::discriminator().is_some());
        assert!(super::Person::discriminator().is_some());
        assert!(super::Program::discriminator().is_some());
        assert!(super::Application::discriminator().is_some());
        assert!(super::Document::discriminator().is_some());
        assert!(super::Email::discriminator().is_some());
        assert!(super::Note::discriminator().is_some());
    }

    #[test]
    fn custom_field_round_trips_every_yaml_scalar_shape_through_json_value() {
        // The `custom` map is typed `BTreeMap<String, serde_json::Value>`
        // but on-disk frontmatter is YAML. This test pins down that
        // serde_yaml's deserializer drives `serde_json::Value`'s visitor
        // correctly for every shape a real eduport custom property
        // could land in: string, integer, float, bool, null, list,
        // nested object.
        let yaml = r#"
name: My note
tags:
  - eduport-type/note
custom_text: hello
custom_int: 42
custom_float: 2.5
custom_bool: true
custom_null: null
custom_list:
  - english
  - german
custom_date: "2026-05-10"
custom_nested:
  inner: value
"#;
        let e = Entity::from_yaml(yaml).unwrap();
        let n = match e {
            Entity::Note(n) => n,
            _ => panic!("expected Note"),
        };

        assert_eq!(
            n.custom.get("custom_text"),
            Some(&serde_json::json!("hello"))
        );
        assert_eq!(n.custom.get("custom_int"), Some(&serde_json::json!(42)));
        assert_eq!(n.custom.get("custom_float"), Some(&serde_json::json!(2.5)));
        assert_eq!(n.custom.get("custom_bool"), Some(&serde_json::json!(true)));
        assert_eq!(n.custom.get("custom_null"), Some(&serde_json::Value::Null));
        assert_eq!(
            n.custom.get("custom_list"),
            Some(&serde_json::json!(["english", "german"]))
        );
        assert_eq!(
            n.custom.get("custom_date"),
            Some(&serde_json::json!("2026-05-10"))
        );
        assert_eq!(
            n.custom.get("custom_nested"),
            Some(&serde_json::json!({ "inner": "value" }))
        );

        // Re-serialise to YAML and reparse: the on-disk write path
        // also has to survive the JSON-typed custom map.
        let yaml2 = Entity::Note(n.clone()).to_yaml().unwrap();
        let back = Entity::from_yaml(&yaml2).unwrap();
        let back = match back {
            Entity::Note(n) => n,
            _ => panic!("expected Note"),
        };
        assert_eq!(back.custom, n.custom);
    }

    #[test]
    fn email_serialises_from_with_alias() {
        let e = Email {
            name: "msg-001".into(),
            tags: vec!["eduport-type/email".into()],
            direction: EmailDirection::Inbound,
            date: "2026-05-10".into(),
            subject: "Hi".into(),
            from: "alice@example.com".into(),
            to: vec!["bob@example.com".into()],
            cc: vec![],
            bcc: vec![],
            related_program: None,
            related_application: None,
            related_people: vec![],
            in_reply_to: None,
            attachments: vec![],
            custom: BTreeMap::new(),
        };
        let yaml = serde_yaml::to_string(&e).unwrap();
        assert!(yaml.contains("from: "));
        let back: Email = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn document_normalize_status_picks_received_when_file_set() {
        let mut d = Document {
            name: "transcript".into(),
            tags: vec!["eduport-type/document".into()],
            title: "Transcript".into(),
            date: None,
            file: Some("transcripts/foo.pdf".into()),
            status: None,
            requested_at: None,
            recommender: None,
            custom: BTreeMap::new(),
        };
        d.normalize_status();
        assert_eq!(d.status, Some(DocumentStatus::Received));
    }

    #[test]
    fn document_normalize_status_picks_drafting_when_no_file() {
        let mut d = Document {
            name: "transcript".into(),
            tags: vec!["eduport-type/document".into()],
            title: "Transcript".into(),
            date: None,
            file: None,
            status: None,
            requested_at: None,
            recommender: None,
            custom: BTreeMap::new(),
        };
        d.normalize_status();
        assert_eq!(d.status, Some(DocumentStatus::Drafting));
    }

    #[test]
    fn user_tags_excludes_eduport_managed_prefixes() {
        let b = BaseEntityFields {
            name: "x".into(),
            tags: vec![
                "eduport-type/note".into(),
                "topic/ai".into(),
                "eduport-doctype/transcript".into(),
                "important".into(),
            ],
        };
        let user = b.user_tags();
        assert_eq!(user, vec!["topic/ai", "important"]);
    }
}
