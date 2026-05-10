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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub custom: BTreeMap<String, serde_yaml::Value>,
}

// ─── Lab ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub custom: BTreeMap<String, serde_yaml::Value>,
}

// ─── Person ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub custom: BTreeMap<String, serde_yaml::Value>,
}

// ─── Program ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProgramLevel {
    Undergrad,
    Masters,
    Phd,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<ProgramLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub department: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    /// ISO date string `YYYY-MM-DD`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tuition: Option<String>,
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
    pub custom: BTreeMap<String, serde_yaml::Value>,
}

// ─── Application ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub custom: BTreeMap<String, serde_yaml::Value>,
}

// ─── Document ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentStatus {
    Requested,
    Drafting,
    Received,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub custom: BTreeMap<String, serde_yaml::Value>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmailDirection {
    Inbound,
    Outbound,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub custom: BTreeMap<String, serde_yaml::Value>,
}

// ─── Note ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(flatten)]
    pub custom: BTreeMap<String, serde_yaml::Value>,
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
