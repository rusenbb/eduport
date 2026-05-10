//! Property variants — the eight types a user can declare on an entity.
//!
//! All variants share a small common header (`key`, `name`, `description`,
//! `required`) plus type-specific fields. Serialised under serde's
//! `#[serde(tag = "type")]` so the YAML/JSON wire format matches the
//! Pydantic original (`type: text`, `type: single-select`, etc.).

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use crate::EntityType;

/// Restricted set of UI tints for select-option chips. Keep this list in
/// sync with the frontend's option-colour palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OptionColor {
    #[default]
    Gray,
    Red,
    Orange,
    Yellow,
    Green,
    Teal,
    Blue,
    Purple,
    Pink,
}

/// One option in a single-select / multi-select property's option list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    #[serde(default)]
    pub color: OptionColor,
}

fn key_re() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"^[a-z][a-z0-9_]{0,63}$").unwrap())
}

fn option_value_re() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"^[a-z0-9][a-z0-9_-]{0,63}$").unwrap())
}

/// Validate a property `key` against the documented shape.
pub fn validate_property_key(key: &str) -> Result<(), String> {
    if !key_re().is_match(key) {
        return Err(format!(
            "property key must match [a-z][a-z0-9_]{{0,63}} (got {:?})",
            key
        ));
    }
    Ok(())
}

/// Validate a select-option `value` against the documented shape.
pub fn validate_option_value(value: &str) -> Result<(), String> {
    if !option_value_re().is_match(value) {
        return Err(format!(
            "option value must match [a-z0-9][a-z0-9_-]{{0,63}} (got {:?})",
            value
        ));
    }
    Ok(())
}

/// Tag identifying which property variant we have. Mirrors the
/// `type` field in the YAML wire format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PropertyKind {
    Text,
    Number,
    Date,
    Checkbox,
    SingleSelect,
    MultiSelect,
    Url,
    Relation,
}

impl PropertyKind {
    /// Wire-format name. Matches the kebab-case rename used by serde
    /// and the `properties.type` column in the SQLite index.
    pub fn as_str(&self) -> &'static str {
        match self {
            PropertyKind::Text => "text",
            PropertyKind::Number => "number",
            PropertyKind::Date => "date",
            PropertyKind::Checkbox => "checkbox",
            PropertyKind::SingleSelect => "single-select",
            PropertyKind::MultiSelect => "multi-select",
            PropertyKind::Url => "url",
            PropertyKind::Relation => "relation",
        }
    }
}

/// A user-declared property on an entity type. Tagged enum: the `type`
/// field on the YAML side picks which variant to deserialise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Property {
    Text(TextProperty),
    Number(NumberProperty),
    Date(DateProperty),
    Checkbox(CheckboxProperty),
    SingleSelect(SingleSelectProperty),
    MultiSelect(MultiSelectProperty),
    Url(UrlProperty),
    Relation(RelationProperty),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextProperty {
    pub key: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumberProperty {
    pub key: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DateProperty {
    pub key: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    /// ISO date string `YYYY-MM-DD`. Validated at use sites; we keep
    /// the on-the-wire shape as a String to match Python's behaviour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckboxProperty {
    pub key: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SingleSelectProperty {
    pub key: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub options: Vec<SelectOption>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MultiSelectProperty {
    pub key: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub options: Vec<SelectOption>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UrlProperty {
    pub key: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationProperty {
    pub key: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    /// `None` (or absent) means any entity type is permitted as the target.
    /// An empty list is rejected — use `None` instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_types: Option<Vec<EntityType>>,
    /// Default reference, in `[[target]]` wikilink form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

impl Property {
    /// The property's user-defined key (the YAML field name on entity
    /// records).
    pub fn key(&self) -> &str {
        match self {
            Property::Text(p) => &p.key,
            Property::Number(p) => &p.key,
            Property::Date(p) => &p.key,
            Property::Checkbox(p) => &p.key,
            Property::SingleSelect(p) => &p.key,
            Property::MultiSelect(p) => &p.key,
            Property::Url(p) => &p.key,
            Property::Relation(p) => &p.key,
        }
    }

    pub fn kind(&self) -> PropertyKind {
        match self {
            Property::Text(_) => PropertyKind::Text,
            Property::Number(_) => PropertyKind::Number,
            Property::Date(_) => PropertyKind::Date,
            Property::Checkbox(_) => PropertyKind::Checkbox,
            Property::SingleSelect(_) => PropertyKind::SingleSelect,
            Property::MultiSelect(_) => PropertyKind::MultiSelect,
            Property::Url(_) => PropertyKind::Url,
            Property::Relation(_) => PropertyKind::Relation,
        }
    }

    /// Validate type-specific invariants that serde can't check on its
    /// own (e.g. duplicate option values, default-not-in-options for
    /// single-select / multi-select, ISO-date format for date defaults,
    /// non-empty target_types for relation properties).
    pub fn validate(&self) -> Result<(), String> {
        validate_property_key(self.key())?;
        match self {
            Property::Date(p) => {
                if let Some(d) = &p.default {
                    parse_iso_date(d).map_err(|e| {
                        format!("default {:?} must be ISO date YYYY-MM-DD: {}", d, e)
                    })?;
                }
            }
            Property::SingleSelect(p) => {
                let mut seen = std::collections::HashSet::new();
                for opt in &p.options {
                    validate_option_value(&opt.value)?;
                    if !seen.insert(&opt.value) {
                        return Err(format!("duplicate option value: {:?}", opt.value));
                    }
                }
                if let Some(d) = &p.default
                    && !p.options.iter().any(|o| &o.value == d)
                {
                    return Err(format!("default {:?} is not among option values", d));
                }
            }
            Property::MultiSelect(p) => {
                let mut seen = std::collections::HashSet::new();
                for opt in &p.options {
                    validate_option_value(&opt.value)?;
                    if !seen.insert(&opt.value) {
                        return Err(format!("duplicate option value: {:?}", opt.value));
                    }
                }
                if let Some(defaults) = &p.default {
                    for d in defaults {
                        if !p.options.iter().any(|o| &o.value == d) {
                            return Err(format!("default {:?} is not among option values", d));
                        }
                    }
                }
            }
            Property::Relation(p) => {
                if let Some(targets) = &p.target_types
                    && targets.is_empty()
                {
                    return Err(
                        "target_types must be omitted entirely or contain at least one entity type"
                            .into(),
                    );
                }
            }
            Property::Url(p) => {
                if let Some(d) = &p.default {
                    url::Url::parse(d)
                        .map_err(|e| format!("default url {:?} is invalid: {}", d, e))?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn parse_iso_date(s: &str) -> Result<(), String> {
    // YYYY-MM-DD strict shape; we don't pull chrono in just for this.
    if s.len() != 10 {
        return Err(format!("expected 10 chars, got {}", s.len()));
    }
    let bytes = s.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return Err("expected YYYY-MM-DD with dashes at positions 4 and 7".into());
    }
    for &i in &[0, 1, 2, 3, 5, 6, 8, 9] {
        if !bytes[i].is_ascii_digit() {
            return Err(format!("non-digit at position {}", i));
        }
    }
    let month: u8 = s[5..7].parse().map_err(|_| "month parse".to_string())?;
    let day: u8 = s[8..10].parse().map_err(|_| "day parse".to_string())?;
    if !(1..=12).contains(&month) {
        return Err(format!("month {} out of range", month));
    }
    if !(1..=31).contains(&day) {
        return Err(format!("day {} out of range", day));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_text_property_through_yaml() {
        let p = Property::Text(TextProperty {
            key: "summary".into(),
            name: "Summary".into(),
            description: None,
            required: false,
            default: None,
        });
        let y = serde_yaml::to_string(&p).unwrap();
        assert!(y.contains("type: text"));
        let back: Property = serde_yaml::from_str(&y).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn round_trip_relation_property_with_target_types() {
        let p = Property::Relation(RelationProperty {
            key: "advisor".into(),
            name: "Advisor".into(),
            description: None,
            required: false,
            target_types: Some(vec![EntityType::Person, EntityType::Lab]),
            default: None,
        });
        let y = serde_yaml::to_string(&p).unwrap();
        let back: Property = serde_yaml::from_str(&y).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn validate_rejects_bad_key() {
        let p = Property::Text(TextProperty {
            key: "Bad-Key".into(),
            name: "n".into(),
            description: None,
            required: false,
            default: None,
        });
        assert!(p.validate().is_err());
    }

    #[test]
    fn validate_rejects_duplicate_option_values() {
        let p = Property::SingleSelect(SingleSelectProperty {
            key: "status".into(),
            name: "Status".into(),
            description: None,
            required: false,
            options: vec![
                SelectOption {
                    value: "draft".into(),
                    label: "Draft".into(),
                    color: OptionColor::Gray,
                },
                SelectOption {
                    value: "draft".into(),
                    label: "Also draft".into(),
                    color: OptionColor::Red,
                },
            ],
            default: None,
        });
        assert!(p.validate().is_err());
    }

    #[test]
    fn validate_rejects_default_not_in_options() {
        let p = Property::SingleSelect(SingleSelectProperty {
            key: "status".into(),
            name: "Status".into(),
            description: None,
            required: false,
            options: vec![SelectOption {
                value: "draft".into(),
                label: "Draft".into(),
                color: OptionColor::Gray,
            }],
            default: Some("ghost".into()),
        });
        assert!(p.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_target_types_list() {
        let p = Property::Relation(RelationProperty {
            key: "rel".into(),
            name: "Rel".into(),
            description: None,
            required: false,
            target_types: Some(vec![]),
            default: None,
        });
        assert!(p.validate().is_err());
    }

    #[test]
    fn validate_rejects_bad_iso_date_default() {
        let p = Property::Date(DateProperty {
            key: "due".into(),
            name: "Due".into(),
            description: None,
            required: false,
            default: Some("13/05/2026".into()),
        });
        assert!(p.validate().is_err());
    }

    #[test]
    fn validate_rejects_bad_url_default() {
        let p = Property::Url(UrlProperty {
            key: "homepage".into(),
            name: "Home".into(),
            description: None,
            required: false,
            default: Some("not a url".into()),
        });
        assert!(p.validate().is_err());
    }

    #[test]
    fn property_kind_round_trips() {
        for kind in [
            PropertyKind::Text,
            PropertyKind::Number,
            PropertyKind::Date,
            PropertyKind::Checkbox,
            PropertyKind::SingleSelect,
            PropertyKind::MultiSelect,
            PropertyKind::Url,
            PropertyKind::Relation,
        ] {
            let yaml = serde_yaml::to_string(&kind).unwrap();
            let back: PropertyKind = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(back, kind);
        }
    }
}
