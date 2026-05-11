//! User-saved list/table/board views.
//!
//! A view captures a configuration of a list-style surface — entity
//! type, filter, sort, group-by, view kind, and view-kind-specific
//! settings. Stored in `<vault>/.eduport/views.yaml`.

pub mod filter_tree;
pub mod store;
pub mod types;

pub use filter_tree::{
    Combinator, FilterCondition, FilterGroup, FilterNode, FilterOperator, FilterTree, FilterValue,
    tree_to_expr,
};
pub use store::{ViewStore, ViewStoreError};
pub use types::{
    SortDir, TypeViews, VIEWS_VERSION, View, ViewFilter, ViewKind, ViewsFile, empty_views_file,
};
