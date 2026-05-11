//! Tree-shaped filter (Notion-style compound filters).
//!
//! The legacy [`crate::view::ViewFilter`] is a flat record-of-records
//! (one map per scalar type, joined by implicit AND). It can't express
//! `email is_not_empty AND name starts_with "X"`, let alone OR or
//! nested groups, because every condition is keyed by property and
//! every operator is bolted to the property type.
//!
//! This module is the new shape:
//!
//! ```text
//! FilterTree
//!  └─ Group(And/Or)
//!       ├─ Condition(property_key, operator, value)
//!       ├─ Condition(...)
//!       └─ Condition(...)
//! ```
//!
//! Phase-B scope is one group level (no nested groups within groups);
//! the [`FilterNode`] enum already supports nesting so adding a UI for
//! it later is purely a frontend change.
//!
//! ## Translator
//!
//! [`tree_to_expr`] walks a `FilterTree` and emits a `vaultdb_core::Expr`.
//! All operator → predicate mappings are 1:1 with vaultdb's existing
//! AST (see `STATE_OF_THE_UNION.md` §1.1) — no vaultdb extensions
//! needed for the operator catalog below.

use serde::{Deserialize, Serialize};

use vaultdb_core::{CompareOp, Expr, Predicate, Value};

/// Top-level filter shape. `None` = no filter (matches everything
/// after the type-tag pin).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilterTree {
    pub root: Option<FilterNode>,
}

/// Either a group (combinator + children) or a leaf condition. The
/// recursive shape supports arbitrary nesting; the Phase-B UI only
/// produces single-group trees.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum FilterNode {
    Group(FilterGroup),
    Cond(FilterCondition),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Combinator {
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilterGroup {
    pub op: Combinator,
    pub children: Vec<FilterNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilterCondition {
    pub property_key: String,
    pub operator: FilterOperator,
    /// `None` for unary operators (`IsEmpty`, `IsNotEmpty`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<FilterValue>,
}

/// Cross-property-type operator catalogue. Not every operator is
/// valid for every property type — the frontend's operator-menu
/// enforces that, and an invalid combo produced by hand-edited YAML
/// is silently ignored by the translator (the predicate just won't
/// match anything).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOperator {
    // Common
    Equals,
    NotEquals,
    IsEmpty,
    IsNotEmpty,

    // Text
    Contains,
    NotContains,
    StartsWith,
    EndsWith,

    // Number / date
    Gt,
    Gte,
    Lt,
    Lte,

    // Multi-select / list
    ContainsAny,
    DoesNotContain,
}

/// Side-channel typed value. Mirrors the operator-vs-value contract
/// in vaultdb's `Predicate::*` shapes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FilterValue {
    Text(String),
    Number(f64),
    Bool(bool),
    /// For `ContainsAny` / `DoesNotContain` — a multi-value match.
    List(Vec<String>),
}

impl FilterValue {
    fn as_text(&self) -> Option<&str> {
        match self {
            FilterValue::Text(s) => Some(s),
            _ => None,
        }
    }
    fn as_list(&self) -> Option<&[String]> {
        match self {
            FilterValue::List(items) => Some(items),
            _ => None,
        }
    }
}

/// Translate the user-built tree into a `vaultdb_core::Expr` that can
/// drop into a [`vaultdb_core::Query`]. Returns `None` for a tree
/// that's empty / all-unsupported — callers should treat that as "no
/// extra filter" and skip the AND-merge with the type-tag predicate.
pub fn tree_to_expr(tree: &FilterTree) -> Option<Expr> {
    tree.root.as_ref().and_then(node_to_expr)
}

fn node_to_expr(node: &FilterNode) -> Option<Expr> {
    match node {
        FilterNode::Group(g) => {
            let exprs: Vec<Expr> = g.children.iter().filter_map(node_to_expr).collect();
            if exprs.is_empty() {
                return None;
            }
            if exprs.len() == 1 {
                return Some(exprs.into_iter().next().unwrap());
            }
            Some(match g.op {
                Combinator::And => Expr::And(exprs),
                Combinator::Or => Expr::Or(exprs),
            })
        }
        FilterNode::Cond(c) => condition_to_expr(c),
    }
}

fn condition_to_expr(c: &FilterCondition) -> Option<Expr> {
    let field = c.property_key.clone();
    Some(match c.operator {
        FilterOperator::Equals => Expr::Predicate(Predicate::Equals {
            field,
            value: filter_value_to_vault(c.value.as_ref()?)?,
        }),
        FilterOperator::NotEquals => Expr::Not(Box::new(Expr::Predicate(Predicate::Equals {
            field,
            value: filter_value_to_vault(c.value.as_ref()?)?,
        }))),
        FilterOperator::IsEmpty => Expr::Predicate(Predicate::Missing { field }),
        FilterOperator::IsNotEmpty => Expr::Predicate(Predicate::Exists { field }),
        FilterOperator::Contains => {
            // For text properties: substring match (vaultdb's
            // Contains over a String value is substring search).
            // For multi-select properties: list-membership (Contains
            // over a List value). The frontend picks the right
            // FilterValue; the translator just forwards.
            Expr::Predicate(Predicate::Contains {
                field,
                value: filter_value_to_vault(c.value.as_ref()?)?,
            })
        }
        FilterOperator::NotContains => Expr::Not(Box::new(Expr::Predicate(Predicate::Contains {
            field,
            value: filter_value_to_vault(c.value.as_ref()?)?,
        }))),
        FilterOperator::StartsWith => Expr::Predicate(Predicate::StartsWith {
            field,
            value: c.value.as_ref()?.as_text()?.to_string(),
        }),
        FilterOperator::EndsWith => Expr::Predicate(Predicate::EndsWith {
            field,
            value: c.value.as_ref()?.as_text()?.to_string(),
        }),
        FilterOperator::Gt => compare(field, CompareOp::Gt, c.value.as_ref()?)?,
        FilterOperator::Gte => compare(field, CompareOp::Ge, c.value.as_ref()?)?,
        FilterOperator::Lt => compare(field, CompareOp::Lt, c.value.as_ref()?)?,
        FilterOperator::Lte => compare(field, CompareOp::Le, c.value.as_ref()?)?,
        FilterOperator::ContainsAny => {
            // Disjunction over each value. For a multi-select
            // property, each Contains is list-membership.
            let items = c.value.as_ref()?.as_list()?;
            if items.is_empty() {
                return None;
            }
            let preds: Vec<Expr> = items
                .iter()
                .map(|v| {
                    Expr::Predicate(Predicate::Contains {
                        field: field.clone(),
                        value: Value::String(v.clone()),
                    })
                })
                .collect();
            if preds.len() == 1 {
                preds.into_iter().next().unwrap()
            } else {
                Expr::Or(preds)
            }
        }
        FilterOperator::DoesNotContain => {
            let items = c.value.as_ref()?.as_list()?;
            if items.is_empty() {
                return None;
            }
            // NOT (any contains) — mirrors Notion's "does not
            // contain any of"
            let preds: Vec<Expr> = items
                .iter()
                .map(|v| {
                    Expr::Predicate(Predicate::Contains {
                        field: field.clone(),
                        value: Value::String(v.clone()),
                    })
                })
                .collect();
            let inner = if preds.len() == 1 {
                preds.into_iter().next().unwrap()
            } else {
                Expr::Or(preds)
            };
            Expr::Not(Box::new(inner))
        }
    })
}

fn compare(field: String, op: CompareOp, value: &FilterValue) -> Option<Expr> {
    Some(Expr::Predicate(Predicate::Compare {
        field,
        op,
        value: filter_value_to_vault(value)?,
    }))
}

fn filter_value_to_vault(v: &FilterValue) -> Option<Value> {
    Some(match v {
        FilterValue::Text(s) => Value::String(s.clone()),
        FilterValue::Number(n) => {
            // Prefer i64 when the user typed an integer — keeps
            // YAML-on-disk friendly (no unnecessary `.0`) and lets
            // vaultdb's cross-numeric coercion do its thing.
            if n.fract() == 0.0 && *n >= i64::MIN as f64 && *n <= i64::MAX as f64 {
                Value::Integer(*n as i64)
            } else {
                Value::Float(*n)
            }
        }
        FilterValue::Bool(b) => Value::Bool(*b),
        FilterValue::List(_) => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cond(key: &str, op: FilterOperator, value: Option<FilterValue>) -> FilterNode {
        FilterNode::Cond(FilterCondition {
            property_key: key.into(),
            operator: op,
            value,
        })
    }

    #[test]
    fn empty_tree_yields_none() {
        let tree = FilterTree { root: None };
        assert!(tree_to_expr(&tree).is_none());
    }

    #[test]
    fn single_equals_emits_equals_predicate() {
        let tree = FilterTree {
            root: Some(cond(
                "country",
                FilterOperator::Equals,
                Some(FilterValue::Text("usa".into())),
            )),
        };
        match tree_to_expr(&tree).unwrap() {
            Expr::Predicate(Predicate::Equals { field, value }) => {
                assert_eq!(field, "country");
                assert_eq!(value, Value::String("usa".into()));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn and_group_with_two_conditions_emits_and() {
        let tree = FilterTree {
            root: Some(FilterNode::Group(FilterGroup {
                op: Combinator::And,
                children: vec![
                    cond(
                        "email",
                        FilterOperator::IsNotEmpty,
                        None,
                    ),
                    cond(
                        "name",
                        FilterOperator::StartsWith,
                        Some(FilterValue::Text("st".into())),
                    ),
                ],
            })),
        };
        match tree_to_expr(&tree).unwrap() {
            Expr::And(children) => {
                assert_eq!(children.len(), 2);
                assert!(matches!(
                    children[0],
                    Expr::Predicate(Predicate::Exists { .. })
                ));
                assert!(matches!(
                    children[1],
                    Expr::Predicate(Predicate::StartsWith { .. })
                ));
            }
            other => panic!("expected And, got {other:?}"),
        }
    }

    #[test]
    fn or_group_emits_or() {
        let tree = FilterTree {
            root: Some(FilterNode::Group(FilterGroup {
                op: Combinator::Or,
                children: vec![
                    cond("country", FilterOperator::Equals, Some(FilterValue::Text("usa".into()))),
                    cond("country", FilterOperator::Equals, Some(FilterValue::Text("japan".into()))),
                ],
            })),
        };
        assert!(matches!(tree_to_expr(&tree).unwrap(), Expr::Or(c) if c.len() == 2));
    }

    #[test]
    fn not_equals_wraps_equals_in_not() {
        let tree = FilterTree {
            root: Some(cond(
                "status",
                FilterOperator::NotEquals,
                Some(FilterValue::Text("rejected".into())),
            )),
        };
        match tree_to_expr(&tree).unwrap() {
            Expr::Not(inner) => {
                assert!(matches!(*inner, Expr::Predicate(Predicate::Equals { .. })));
            }
            other => panic!("expected Not, got {other:?}"),
        }
    }

    #[test]
    fn is_empty_emits_missing() {
        let tree = FilterTree {
            root: Some(cond("city", FilterOperator::IsEmpty, None)),
        };
        assert!(matches!(
            tree_to_expr(&tree).unwrap(),
            Expr::Predicate(Predicate::Missing { .. })
        ));
    }

    #[test]
    fn contains_any_with_one_value_collapses_to_single_predicate() {
        let tree = FilterTree {
            root: Some(cond(
                "language",
                FilterOperator::ContainsAny,
                Some(FilterValue::List(vec!["english".into()])),
            )),
        };
        // One value -> single Contains, no Or wrapper.
        assert!(matches!(
            tree_to_expr(&tree).unwrap(),
            Expr::Predicate(Predicate::Contains { .. })
        ));
    }

    #[test]
    fn contains_any_with_multiple_values_emits_or_of_contains() {
        let tree = FilterTree {
            root: Some(cond(
                "language",
                FilterOperator::ContainsAny,
                Some(FilterValue::List(vec!["english".into(), "german".into()])),
            )),
        };
        match tree_to_expr(&tree).unwrap() {
            Expr::Or(c) => {
                assert_eq!(c.len(), 2);
                for e in c {
                    assert!(matches!(e, Expr::Predicate(Predicate::Contains { .. })));
                }
            }
            other => panic!("expected Or, got {other:?}"),
        }
    }

    #[test]
    fn integer_number_round_trips_as_i64() {
        let tree = FilterTree {
            root: Some(cond(
                "tuition",
                FilterOperator::Gt,
                Some(FilterValue::Number(5000.0)),
            )),
        };
        match tree_to_expr(&tree).unwrap() {
            Expr::Predicate(Predicate::Compare { value, op, .. }) => {
                assert_eq!(op, CompareOp::Gt);
                assert_eq!(value, Value::Integer(5000));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn fractional_number_stays_f64() {
        let tree = FilterTree {
            root: Some(cond(
                "tuition",
                FilterOperator::Lte,
                Some(FilterValue::Number(1234.56)),
            )),
        };
        match tree_to_expr(&tree).unwrap() {
            Expr::Predicate(Predicate::Compare { value, .. }) => {
                assert_eq!(value, Value::Float(1234.56));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn round_trips_through_yaml() {
        let tree = FilterTree {
            root: Some(FilterNode::Group(FilterGroup {
                op: Combinator::And,
                children: vec![
                    cond("email", FilterOperator::IsNotEmpty, None),
                    cond(
                        "name",
                        FilterOperator::StartsWith,
                        Some(FilterValue::Text("Stanford".into())),
                    ),
                ],
            })),
        };
        let y = serde_yaml::to_string(&tree).unwrap();
        let back: FilterTree = serde_yaml::from_str(&y).unwrap();
        assert_eq!(back, tree);
    }
}
