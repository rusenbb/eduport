//! The eight entity types that eduport recognises. Frozen list — adding
//! a ninth requires touching every store, parser, and UI surface, so we
//! keep it explicit. Kept here at the crate root because Settings,
//! Schema, View, and the entity parsers all reference it.

use serde::{Deserialize, Serialize};

/// Eduport's eight first-class entity types. Stored in record frontmatter
/// as a `eduport-type/<value>` tag (e.g. `eduport-type/university`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum EntityType {
    University,
    Lab,
    Person,
    Program,
    Application,
    Document,
    Email,
    Note,
}

impl EntityType {
    /// All variants in declaration order. Use this for "every type must
    /// have an entry" invariants in Schema and ViewsFile.
    pub const ALL: [EntityType; 8] = [
        EntityType::University,
        EntityType::Lab,
        EntityType::Person,
        EntityType::Program,
        EntityType::Application,
        EntityType::Document,
        EntityType::Email,
        EntityType::Note,
    ];

    /// Lowercase string form, matching the YAML/TOML/JSON wire format.
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityType::University => "university",
            EntityType::Lab => "lab",
            EntityType::Person => "person",
            EntityType::Program => "program",
            EntityType::Application => "application",
            EntityType::Document => "document",
            EntityType::Email => "email",
            EntityType::Note => "note",
        }
    }
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for EntityType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for t in EntityType::ALL {
            if t.as_str() == s {
                return Ok(t);
            }
        }
        Err(format!("unknown entity type: {}", s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_every_variant_through_serde_yaml() {
        for t in EntityType::ALL {
            let yaml = serde_yaml::to_string(&t).unwrap();
            let back: EntityType = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(back, t);
        }
    }

    #[test]
    fn from_str_round_trip() {
        for t in EntityType::ALL {
            assert_eq!(t.as_str().parse::<EntityType>().unwrap(), t);
        }
        assert!("nonexistent".parse::<EntityType>().is_err());
    }
}
