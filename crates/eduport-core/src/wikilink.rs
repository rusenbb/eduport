//! WikiLink — a typed wrapper over the `[[target]]` reference shape.
//!
//! Round-trips through YAML/JSON as the bracketed string form, so the
//! surrounding entity's serialized frontmatter matches the on-disk
//! markdown text. `target` is the filename stem (no `.md`, no
//! brackets).

use std::sync::OnceLock;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

fn wikilink_re() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"^\[\[([^\]\[]+)\]\]$").unwrap())
}

/// `[[target]]` reference. Wraps a filename stem (no `.md`, no
/// brackets). Equality is by `target`.
///
/// `#[specta(transparent)]` tells the TS-binding generator to render
/// this as a plain `string` — matching the wire format
/// (`"[[target]]"`) and aligning with how the frontend already
/// handles wikilink values throughout the codebase.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, specta::Type)]
#[specta(transparent)]
pub struct WikiLink {
    pub target: String,
}

impl WikiLink {
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
        }
    }

    /// Parse a string as a wikilink, returning None if it's not in
    /// `[[...]]` form.
    pub fn parse(s: &str) -> Option<Self> {
        let caps = wikilink_re().captures(s.trim())?;
        let inner = caps.get(1)?.as_str().trim().to_string();
        if inner.is_empty() {
            return None;
        }
        Some(Self { target: inner })
    }
}

impl std::fmt::Display for WikiLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[[{}]]", self.target)
    }
}

impl Serialize for WikiLink {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("[[{}]]", self.target))
    }
}

impl<'de> Deserialize<'de> for WikiLink {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        WikiLink::parse(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("not a wikilink: {:?}", s)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trips() {
        let w = WikiLink::parse("[[Stanford University]]").unwrap();
        assert_eq!(w.target, "Stanford University");
        assert_eq!(format!("{}", w), "[[Stanford University]]");
    }

    #[test]
    fn parse_rejects_bad_shapes() {
        assert!(WikiLink::parse("Stanford").is_none());
        assert!(WikiLink::parse("[Stanford]").is_none());
        assert!(WikiLink::parse("[[]]").is_none());
        assert!(WikiLink::parse("[[a][b]]").is_none()); // square brackets in inner
    }

    #[test]
    fn round_trip_through_yaml() {
        let w = WikiLink::new("Foo");
        let y = serde_yaml::to_string(&w).unwrap();
        // serde_yaml escapes the brackets; round-trip must give us back
        // the same WikiLink.
        let back: WikiLink = serde_yaml::from_str(&y).unwrap();
        assert_eq!(back, w);
    }

    #[test]
    fn deserialize_rejects_non_wikilink_string() {
        let r: Result<WikiLink, _> = serde_yaml::from_str("'plain string'");
        assert!(r.is_err());
    }
}
