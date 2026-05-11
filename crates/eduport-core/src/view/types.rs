//! View types — saved configurations of an entity-type list-surface.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::EntityType;
use crate::view::filter_tree::FilterTree;

pub const VIEWS_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewKind {
    #[default]
    List,
    Table,
    Board,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDir {
    #[default]
    Asc,
    Desc,
}

fn id_re() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"^[a-z0-9][a-zA-Z0-9_-]{0,127}$").unwrap())
}

/// Frontend-shaped property filter — mirrors the request body of the
/// (legacy) `/api/properties/filter` endpoint.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewFilter {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub text: BTreeMap<String, String>,
    /// Numeric range: `(min, max)` — either side optional.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub num: BTreeMap<String, (Option<f64>, Option<f64>)>,
    /// Date range as ISO `YYYY-MM-DD` strings — either side optional.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub date: BTreeMap<String, (Option<String>, Option<String>)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: ViewKind,
    /// Legacy flat filter (text equality / num range / date range).
    /// Kept on disk for back-compat; new views can leave this empty
    /// and use [`Self::filter_tree`] instead. The two are merged with
    /// AND when both are populated — useful for evolving an existing
    /// view by *adding* compound conditions without losing the old
    /// chip filter.
    #[serde(default)]
    pub filter: ViewFilter,
    /// Notion-style compound filter (AND/OR groups + per-property
    /// operators). When present, takes precedence over / merges with
    /// `filter`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter_tree: Option<FilterTree>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_key: Option<String>,
    #[serde(default)]
    pub sort_dir: SortDir,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_by_key: Option<String>,
    /// Table view: which property keys to render as columns.
    /// `None` → use a sensible default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<String>>,
    /// Board view: which property keys to render on each card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_properties: Option<Vec<String>>,
}

impl View {
    pub fn validate(&self) -> Result<(), String> {
        if !id_re().is_match(&self.id) {
            return Err(format!(
                "view id must match [a-z0-9][a-zA-Z0-9_-]{{0,127}} (got {:?})",
                self.id
            ));
        }
        if self.name.is_empty() || self.name.len() > 120 {
            return Err(format!(
                "view name length must be 1..=120 (got {})",
                self.name.len()
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeViews {
    #[serde(default)]
    pub views: Vec<View>,
}

impl TypeViews {
    pub fn view(&self, view_id: &str) -> Option<&View> {
        self.views.iter().find(|v| v.id == view_id)
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut seen = std::collections::HashSet::new();
        for v in &self.views {
            v.validate()?;
            if !seen.insert(&v.id) {
                return Err(format!("duplicate view id: {:?}", v.id));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewsFile {
    #[serde(default = "default_version")]
    pub version: u32,
    pub types: BTreeMap<EntityType, TypeViews>,
}

fn default_version() -> u32 {
    VIEWS_VERSION
}

impl ViewsFile {
    pub fn for_type(&self, entity_type: EntityType) -> &TypeViews {
        self.types
            .get(&entity_type)
            .expect("ViewsFile is missing an entity type entry; load_views enforces this invariant")
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.version != VIEWS_VERSION {
            return Err(format!(
                "unsupported views version {}; this build expects {}",
                self.version, VIEWS_VERSION
            ));
        }
        for t in EntityType::ALL {
            if !self.types.contains_key(&t) {
                return Err(format!("views file missing entry for {:?}", t));
            }
        }
        for tv in self.types.values() {
            tv.validate()?;
        }
        Ok(())
    }
}

pub fn empty_views_file() -> ViewsFile {
    let types = EntityType::ALL
        .into_iter()
        .map(|t| (t, TypeViews::default()))
        .collect();
    ViewsFile {
        version: VIEWS_VERSION,
        types,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_views_file_has_all_eight_types() {
        let v = empty_views_file();
        assert_eq!(v.version, VIEWS_VERSION);
        for t in EntityType::ALL {
            assert!(v.types.contains_key(&t));
        }
        v.validate().unwrap();
    }

    #[test]
    fn view_round_trips_through_yaml() {
        let v = View {
            id: "active".into(),
            name: "Active applications".into(),
            kind: ViewKind::Table,
            filter: ViewFilter::default(),
            filter_tree: None,
            sort_key: Some("deadline".into()),
            sort_dir: SortDir::Desc,
            group_by_key: None,
            columns: Some(vec!["status".into(), "deadline".into()]),
            card_properties: None,
        };
        let yaml = serde_yaml::to_string(&v).unwrap();
        let back: View = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(back, v);
        v.validate().unwrap();
    }

    #[test]
    fn validate_rejects_duplicate_view_ids() {
        let v = View {
            id: "x".into(),
            name: "X".into(),
            kind: ViewKind::List,
            filter: ViewFilter::default(),
            filter_tree: None,
            sort_key: None,
            sort_dir: SortDir::Asc,
            group_by_key: None,
            columns: None,
            card_properties: None,
        };
        let tv = TypeViews {
            views: vec![v.clone(), v],
        };
        assert!(tv.validate().is_err());
    }

    #[test]
    fn validate_rejects_bad_view_id() {
        let v = View {
            id: "Bad ID".into(), // contains space
            name: "X".into(),
            kind: ViewKind::List,
            filter: ViewFilter::default(),
            filter_tree: None,
            sort_key: None,
            sort_dir: SortDir::Asc,
            group_by_key: None,
            columns: None,
            card_properties: None,
        };
        assert!(v.validate().is_err());
    }
}
