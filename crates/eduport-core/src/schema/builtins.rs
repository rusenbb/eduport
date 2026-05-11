//! Built-in property seed lists.
//!
//! Eduport's eight entity types each have a small set of system-defined
//! fields (e.g. `country`, `city`, `role`, `tuition`). Historically the
//! frontend hardcoded these in a `FIELD_DEFS` lookup table. That worked
//! for rendering, but it meant **the user could never edit a built-in
//! field's options** — there was no way to add a new country or
//! language to the dropdown without rebuilding the app.
//!
//! The Notion-style affordance the user wants is: built-in selects
//! ship with a curated seed list, and the user can add new options as
//! they encounter them, just like for custom properties. The cleanest
//! way to deliver that is to put the built-in fields *into the
//! schema* alongside custom properties, marked `is_builtin: true`.
//! [`SchemaStore::load`](crate::schema::SchemaStore::load) merges this
//! seed into every vault on first load (and again later for any
//! built-in that's missing — so adopting a new built-in field doesn't
//! require a migration).
//!
//! Built-in properties are protected against `delete_property` and
//! `type` changes; everything else (option list, name, description) is
//! patchable like any other property.

use crate::EntityType;
use crate::schema::property::{
    DateProperty, MultiSelectProperty, NumberProperty, OptionColor, Property, RelationProperty,
    SelectOption, SingleSelectProperty, TextProperty, UrlProperty,
};

/// Built-in property seed list for `entity_type`. Returned in display
/// order (the order the schema editor and the row editor render
/// fields).
///
/// The list mirrors the frontend's historic `FIELD_DEFS` table, but
/// with the property *types* updated per the 2026-05 design pass:
///
/// - `country` is a single-select seeded with the user's primary
///   target countries (USA, Japan, France, Germany, Singapore, South
///   Korea); not exhaustive — the user adds more as they apply.
/// - `city` is a single-select seeded with popular destination cities
///   in those countries.
/// - `language` is a multi-select (programs are often bilingual)
///   seeded with the official languages of the target countries.
/// - `role` is a single-select seeded with the standard academic role
///   ladder (PhD student through full professor, plus admin).
/// - `tuition` is a number (was a free-text string before — couldn't
///   be sorted or filtered numerically).
///
/// Wikilink-shaped fields (`university`, `labs`, `program`,
/// `recommender`, etc.) are seeded as `relation` with the target
/// entity type pinned. Resource lists (`links`, `emails`, `documents`,
/// `attachments`) are not in the schema — they're carried by the
/// typed entity struct as `Vec<LinkResource>` / `Vec<WikiLink>` and
/// have no Notion-style "options" semantics.
pub fn seeded_builtins(entity_type: EntityType) -> Vec<Property> {
    match entity_type {
        EntityType::University => vec![
            country_property(),
            city_property(),
            url_builtin("website", "Website"),
        ],
        EntityType::Lab => vec![
            text_builtin("focus", "Focus"),
            url_builtin("website", "Website"),
            relation_builtin("university", "University", &[EntityType::University]),
        ],
        EntityType::Person => vec![
            role_property(),
            text_builtin("email", "Email"),
            url_builtin("website", "Website"),
            relation_builtin("university", "University", &[EntityType::University]),
            relation_builtin("labs", "Labs", &[EntityType::Lab]),
        ],
        EntityType::Program => vec![
            level_property(),
            text_builtin("department", "Department"),
            language_property(),
            text_builtin("duration", "Duration"),
            date_builtin("deadline", "Deadline"),
            tuition_property(),
            url_builtin("website", "Website"),
            relation_builtin("university", "University", &[EntityType::University]),
            relation_builtin("people", "People", &[EntityType::Person]),
        ],
        EntityType::Application => vec![
            relation_builtin("program", "Program", &[EntityType::Program]),
            status_property(),
            date_builtin("internal_deadline", "Internal deadline"),
            date_builtin("submitted_at", "Submitted at"),
            date_builtin("decision_at", "Decision at"),
            relation_builtin("documents", "Documents", &[EntityType::Document]),
        ],
        EntityType::Document => vec![
            text_builtin("title", "Title"),
            date_builtin("date", "Date"),
            text_builtin("file", "File"),
            doc_status_property(),
            date_builtin("requested_at", "Requested at"),
            relation_builtin("recommender", "Recommender", &[EntityType::Person]),
        ],
        EntityType::Email => vec![
            email_direction_property(),
            date_builtin("date", "Date"),
            text_builtin("subject", "Subject"),
            text_builtin("from", "From"),
            text_builtin("to", "To"),
            text_builtin("cc", "Cc"),
            text_builtin("bcc", "Bcc"),
            relation_builtin("related_program", "Related program", &[EntityType::Program]),
            relation_builtin(
                "related_application",
                "Related application",
                &[EntityType::Application],
            ),
            relation_builtin("related_people", "Related people", &[EntityType::Person]),
            relation_builtin("in_reply_to", "In reply to", &[EntityType::Email]),
            relation_builtin("attachments", "Attachments", &[EntityType::Document]),
        ],
        EntityType::Note => vec![],
    }
}

// ── Property constructors ────────────────────────────────────────────

fn text_builtin(key: &str, name: &str) -> Property {
    Property::Text(TextProperty {
        key: key.into(),
        name: name.into(),
        description: None,
        required: false,
        is_builtin: true,
        default: None,
    })
}

fn date_builtin(key: &str, name: &str) -> Property {
    Property::Date(DateProperty {
        key: key.into(),
        name: name.into(),
        description: None,
        required: false,
        is_builtin: true,
        default: None,
    })
}

fn url_builtin(key: &str, name: &str) -> Property {
    Property::Url(UrlProperty {
        key: key.into(),
        name: name.into(),
        description: None,
        required: false,
        is_builtin: true,
        default: None,
    })
}

fn relation_builtin(key: &str, name: &str, targets: &[EntityType]) -> Property {
    Property::Relation(RelationProperty {
        key: key.into(),
        name: name.into(),
        description: None,
        required: false,
        is_builtin: true,
        target_types: Some(targets.to_vec()),
        default: None,
    })
}

fn opt(value: &str, label: &str, color: OptionColor) -> SelectOption {
    SelectOption {
        value: value.into(),
        label: label.into(),
        color,
    }
}

// ── Specific built-in selects ────────────────────────────────────────

fn country_property() -> Property {
    Property::SingleSelect(SingleSelectProperty {
        key: "country".into(),
        name: "Country".into(),
        description: None,
        required: false,
        is_builtin: true,
        options: vec![
            opt("usa", "USA", OptionColor::Blue),
            opt("japan", "Japan", OptionColor::Red),
            opt("france", "France", OptionColor::Purple),
            opt("germany", "Germany", OptionColor::Yellow),
            opt("singapore", "Singapore", OptionColor::Pink),
            opt("south-korea", "South Korea", OptionColor::Teal),
        ],
        default: None,
    })
}

fn city_property() -> Property {
    Property::SingleSelect(SingleSelectProperty {
        key: "city".into(),
        name: "City".into(),
        description: None,
        required: false,
        is_builtin: true,
        options: vec![
            // USA
            opt("new-york", "New York", OptionColor::Blue),
            opt("boston", "Boston", OptionColor::Blue),
            opt("san-francisco", "San Francisco", OptionColor::Blue),
            opt("los-angeles", "Los Angeles", OptionColor::Blue),
            // Japan
            opt("tokyo", "Tokyo", OptionColor::Red),
            opt("kyoto", "Kyoto", OptionColor::Red),
            opt("osaka", "Osaka", OptionColor::Red),
            // France
            opt("paris", "Paris", OptionColor::Purple),
            opt("lyon", "Lyon", OptionColor::Purple),
            // Germany
            opt("berlin", "Berlin", OptionColor::Yellow),
            opt("munich", "Munich", OptionColor::Yellow),
            opt("heidelberg", "Heidelberg", OptionColor::Yellow),
            // Singapore (city-state)
            opt("singapore", "Singapore", OptionColor::Pink),
            // South Korea
            opt("seoul", "Seoul", OptionColor::Teal),
            opt("busan", "Busan", OptionColor::Teal),
        ],
        default: None,
    })
}

fn language_property() -> Property {
    Property::MultiSelect(MultiSelectProperty {
        key: "language".into(),
        name: "Language".into(),
        description: None,
        required: false,
        is_builtin: true,
        options: vec![
            opt("english", "English", OptionColor::Blue),
            opt("japanese", "Japanese", OptionColor::Red),
            opt("french", "French", OptionColor::Purple),
            opt("german", "German", OptionColor::Yellow),
            opt("mandarin", "Mandarin", OptionColor::Pink),
            opt("malay", "Malay", OptionColor::Pink),
            opt("tamil", "Tamil", OptionColor::Pink),
            opt("korean", "Korean", OptionColor::Teal),
        ],
        default: None,
    })
}

fn role_property() -> Property {
    Property::SingleSelect(SingleSelectProperty {
        key: "role".into(),
        name: "Role".into(),
        description: None,
        required: false,
        is_builtin: true,
        options: vec![
            opt("phd-student", "PhD Student", OptionColor::Blue),
            opt("masters-student", "Master's Student", OptionColor::Teal),
            opt("postdoc", "Postdoc", OptionColor::Green),
            opt("research-scientist", "Research Scientist", OptionColor::Yellow),
            opt("lecturer", "Lecturer", OptionColor::Orange),
            opt("assistant-professor", "Assistant Professor", OptionColor::Purple),
            opt("associate-professor", "Associate Professor", OptionColor::Pink),
            opt("professor", "Professor", OptionColor::Red),
            opt("admin", "Admin", OptionColor::Gray),
        ],
        default: None,
    })
}

fn tuition_property() -> Property {
    Property::Number(NumberProperty {
        key: "tuition".into(),
        name: "Tuition".into(),
        description: None,
        required: false,
        is_builtin: true,
        unit: None,
        default: None,
    })
}

fn level_property() -> Property {
    Property::SingleSelect(SingleSelectProperty {
        key: "level".into(),
        name: "Level".into(),
        description: None,
        required: false,
        is_builtin: true,
        options: vec![
            opt("undergrad", "Undergraduate", OptionColor::Blue),
            opt("masters", "Master's", OptionColor::Teal),
            opt("phd", "PhD", OptionColor::Purple),
        ],
        default: None,
    })
}

fn status_property() -> Property {
    // Mirrors the existing ApplicationStatus enum's variants. The Rust
    // entity struct stays typed as `ApplicationStatus`; this schema
    // entry is what the frontend uses to render a chip and a filter
    // operator menu.
    Property::SingleSelect(SingleSelectProperty {
        key: "status".into(),
        name: "Status".into(),
        description: None,
        required: false,
        is_builtin: true,
        options: vec![
            opt("planning", "Planning", OptionColor::Gray),
            opt("drafting", "Drafting", OptionColor::Yellow),
            opt("submitted", "Submitted", OptionColor::Blue),
            opt("decision-pending", "Decision pending", OptionColor::Orange),
            opt("accepted", "Accepted", OptionColor::Green),
            opt("rejected", "Rejected", OptionColor::Red),
            opt("withdrawn", "Withdrawn", OptionColor::Gray),
        ],
        default: None,
    })
}

fn doc_status_property() -> Property {
    Property::SingleSelect(SingleSelectProperty {
        key: "status".into(),
        name: "Status".into(),
        description: None,
        required: false,
        is_builtin: true,
        options: vec![
            opt("requested", "Requested", OptionColor::Yellow),
            opt("drafting", "Drafting", OptionColor::Orange),
            opt("received", "Received", OptionColor::Green),
        ],
        default: None,
    })
}

fn email_direction_property() -> Property {
    Property::SingleSelect(SingleSelectProperty {
        key: "direction".into(),
        name: "Direction".into(),
        description: None,
        required: false,
        is_builtin: true,
        options: vec![
            opt("inbound", "Inbound", OptionColor::Blue),
            opt("outbound", "Outbound", OptionColor::Green),
        ],
        default: None,
    })
}

/// Returns `true` if `key` is a built-in property key for `entity_type`.
/// Used by `add_property` to reject collisions between user-defined
/// custom properties and system-seeded built-ins.
pub fn is_builtin_key(entity_type: EntityType, key: &str) -> bool {
    seeded_builtins(entity_type).iter().any(|p| p.key() == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_seeded_property_validates() {
        for t in EntityType::ALL {
            for p in seeded_builtins(t) {
                assert!(
                    p.validate().is_ok(),
                    "{:?} built-in {:?} failed to validate: {:?}",
                    t,
                    p.key(),
                    p.validate(),
                );
                assert!(p.is_builtin(), "seeded property must carry is_builtin");
            }
        }
    }

    #[test]
    fn university_seeds_country_as_select_with_curated_options() {
        let props = seeded_builtins(EntityType::University);
        let country = props.iter().find(|p| p.key() == "country").unwrap();
        match country {
            Property::SingleSelect(p) => {
                assert!(p.options.iter().any(|o| o.value == "usa"));
                assert!(p.options.iter().any(|o| o.value == "japan"));
                assert!(p.options.iter().any(|o| o.value == "south-korea"));
            }
            other => panic!("expected single-select for country, got {:?}", other.kind()),
        }
    }

    #[test]
    fn program_seeds_language_as_multi_select() {
        let props = seeded_builtins(EntityType::Program);
        let language = props.iter().find(|p| p.key() == "language").unwrap();
        assert!(matches!(language.kind(), crate::schema::PropertyKind::MultiSelect));
    }

    #[test]
    fn program_seeds_tuition_as_number() {
        let props = seeded_builtins(EntityType::Program);
        let tuition = props.iter().find(|p| p.key() == "tuition").unwrap();
        assert!(matches!(tuition.kind(), crate::schema::PropertyKind::Number));
    }

    #[test]
    fn note_has_no_built_ins() {
        assert!(seeded_builtins(EntityType::Note).is_empty());
    }

    #[test]
    fn is_builtin_key_recognises_seeded_keys() {
        assert!(is_builtin_key(EntityType::Person, "role"));
        assert!(is_builtin_key(EntityType::University, "country"));
        assert!(!is_builtin_key(EntityType::Person, "ghost_field"));
        assert!(!is_builtin_key(EntityType::Note, "anything"));
    }
}
