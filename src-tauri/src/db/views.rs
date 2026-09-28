use std::collections::HashSet;

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Db, DbError, RecordType, RelationDefinition, Result, json_sql_error};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FilterRule {
    pub field: String,
    pub operator: String,
    #[serde(default)]
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SortRule {
    pub field: String,
    pub direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViewConfig {
    pub columns: Vec<String>,
    #[serde(default)]
    pub filters: Vec<FilterRule>,
    #[serde(default)]
    pub sorts: Vec<SortRule>,
    #[serde(default)]
    pub group_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableView {
    pub id: String,
    pub type_key: String,
    pub name: String,
    pub config: ViewConfig,
    pub is_default: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TableViewInput {
    pub id: Option<String>,
    pub type_key: String,
    pub name: String,
    pub config: ViewConfig,
}

pub fn default_view(kind: &RecordType, definitions: &[RelationDefinition]) -> TableView {
    let mut columns = vec!["title".into()];
    columns.extend(kind.fields.iter().map(|field| field.key.clone()));
    columns.extend(
        definitions
            .iter()
            .filter(|definition| {
                definition.source_type == kind.key || definition.target_type == kind.key
            })
            .map(|definition| format!("relation:{}", definition.id)),
    );
    columns.extend(["tags".into(), "updated_at".into()]);
    TableView {
        id: format!("default:{}", kind.key),
        type_key: kind.key.clone(),
        name: "Table".into(),
        config: ViewConfig {
            columns,
            filters: Vec::new(),
            sorts: Vec::new(),
            group_by: None,
        },
        is_default: true,
    }
}

pub(super) fn validate_view(
    kind: &RecordType,
    definitions: &[RelationDefinition],
    name: &str,
    config: &ViewConfig,
) -> Result<()> {
    if name.trim().is_empty() || name.len() > 100 {
        return Err(DbError::Invalid(
            "view name must be 1-100 characters".into(),
        ));
    }
    let allowed: HashSet<String> = ["title", "body", "tags", "updated_at"]
        .into_iter()
        .map(str::to_owned)
        .chain(kind.fields.iter().map(|field| field.key.clone()))
        .chain(
            definitions
                .iter()
                .filter(|definition| {
                    definition.source_type == kind.key || definition.target_type == kind.key
                })
                .map(|definition| format!("relation:{}", definition.id)),
        )
        .collect();
    let mut seen = HashSet::new();
    if config.columns.is_empty()
        || !config.columns.iter().any(|field| field == "title")
        || config
            .columns
            .iter()
            .any(|field| !allowed.contains(field.as_str()) || !seen.insert(field))
    {
        return Err(DbError::Invalid(
            "view columns must be unique known fields and include title".into(),
        ));
    }
    if config.filters.len() > 8 || config.sorts.len() > 8 {
        return Err(DbError::Invalid(
            "a view supports at most eight filters and eight sorts".into(),
        ));
    }
    for filter in &config.filters {
        if !allowed.contains(filter.field.as_str())
            || !matches!(
                filter.operator.as_str(),
                "contains" | "equals" | "not_equals" | "is_empty" | "greater_than" | "less_than"
            )
            || filter.value.len() > 250
        {
            return Err(DbError::Invalid("view filter is invalid".into()));
        }
    }
    for sort in &config.sorts {
        if !allowed.contains(sort.field.as_str())
            || !matches!(sort.direction.as_str(), "asc" | "desc")
        {
            return Err(DbError::Invalid("view sort is invalid".into()));
        }
    }
    if config
        .group_by
        .as_deref()
        .is_some_and(|field| !allowed.contains(field))
    {
        return Err(DbError::Invalid("view group field is unknown".into()));
    }
    Ok(())
}

impl Db {
    pub fn list_views(&self, type_key: &str) -> Result<Vec<TableView>> {
        let kind = self
            .list_types()?
            .into_iter()
            .find(|item| item.key == type_key)
            .ok_or_else(|| DbError::Invalid(format!("unknown type: {type_key}")))?;
        let definitions = self.list_relation_definitions(Some(type_key))?;
        let mut views = vec![default_view(&kind, &definitions)];
        let mut stmt = self.conn.prepare(
            "SELECT id, type_key, name, config_json FROM table_views
             WHERE type_key = ?1 ORDER BY name COLLATE NOCASE, id",
        )?;
        let saved = stmt.query_map([type_key], |row| {
            let config_text: String = row.get(3)?;
            Ok(TableView {
                id: row.get(0)?,
                type_key: row.get(1)?,
                name: row.get(2)?,
                config: serde_json::from_str(&config_text).map_err(json_sql_error)?,
                is_default: false,
            })
        })?;
        views.extend(saved.collect::<rusqlite::Result<Vec<_>>>()?);
        Ok(views)
    }

    pub fn all_custom_views(&self) -> Result<Vec<TableView>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, type_key, name, config_json FROM table_views ORDER BY type_key, name, id",
        )?;
        let rows = stmt.query_map([], |row| {
            let config_text: String = row.get(3)?;
            Ok(TableView {
                id: row.get(0)?,
                type_key: row.get(1)?,
                name: row.get(2)?,
                config: serde_json::from_str(&config_text).map_err(json_sql_error)?,
                is_default: false,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn put_view(&mut self, input: TableViewInput) -> Result<TableView> {
        let kind = self
            .list_types()?
            .into_iter()
            .find(|item| item.key == input.type_key)
            .ok_or_else(|| DbError::Invalid(format!("unknown type: {}", input.type_key)))?;
        let definitions = self.list_relation_definitions(Some(&kind.key))?;
        validate_view(&kind, &definitions, &input.name, &input.config)?;
        let id = match input.id {
            Some(id) => {
                let existing_type: Option<String> = self
                    .conn
                    .query_row(
                        "SELECT type_key FROM table_views WHERE id = ?1",
                        [&id],
                        |row| row.get(0),
                    )
                    .optional()?;
                if existing_type.as_deref() != Some(input.type_key.as_str()) {
                    return Err(DbError::Invalid("view cannot change entity type".into()));
                }
                self.conn.execute(
                    "UPDATE table_views SET name = ?2, config_json = ?3 WHERE id = ?1",
                    params![
                        id,
                        input.name.trim(),
                        serde_json::to_string(&input.config).unwrap()
                    ],
                )?;
                id
            }
            None => {
                let id = Uuid::new_v4().to_string();
                self.conn.execute(
                    "INSERT INTO table_views(id, type_key, name, config_json) VALUES(?1, ?2, ?3, ?4)",
                    params![id, input.type_key, input.name.trim(), serde_json::to_string(&input.config).unwrap()],
                )?;
                id
            }
        };
        self.list_views(&kind.key)?
            .into_iter()
            .find(|view| view.id == id)
            .ok_or(DbError::NotFound)
    }

    pub fn delete_view(&mut self, id: &str) -> Result<()> {
        if self
            .conn
            .execute("DELETE FROM table_views WHERE id = ?1", [id])?
            != 1
        {
            return Err(DbError::NotFound);
        }
        Ok(())
    }
}
