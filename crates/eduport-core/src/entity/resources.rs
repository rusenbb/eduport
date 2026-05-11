//! Small structured resources that appear inside several entity types
//! (`LinkResource`, `EmailResource`). Mirrors the Pydantic originals.

use serde::{Deserialize, Serialize};

use crate::wikilink::WikiLink;

/// A labelled URL — appears on University, Lab, Program, Person.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields)]
pub struct LinkResource {
    pub label: String,
    pub url: String,
}

/// A labelled email address with an optional `Person` wikilink that
/// owns it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields)]
pub struct EmailResource {
    pub label: String,
    pub email: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person: Option<WikiLink>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_resource_round_trip() {
        let lr = LinkResource {
            label: "Stanford CS".into(),
            url: "https://cs.stanford.edu".into(),
        };
        let yaml = serde_yaml::to_string(&lr).unwrap();
        let back: LinkResource = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(back, lr);
    }

    #[test]
    fn email_resource_with_person_round_trip() {
        let er = EmailResource {
            label: "admissions".into(),
            email: "admissions@cs.stanford.edu".into(),
            person: Some(WikiLink::new("Dr. Smith")),
        };
        let yaml = serde_yaml::to_string(&er).unwrap();
        let back: EmailResource = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(back, er);
    }

    #[test]
    fn email_resource_without_person_omits_field() {
        let er = EmailResource {
            label: "info".into(),
            email: "info@x.com".into(),
            person: None,
        };
        let yaml = serde_yaml::to_string(&er).unwrap();
        assert!(!yaml.contains("person"));
    }
}
