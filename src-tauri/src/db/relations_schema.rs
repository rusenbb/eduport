use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Db, DbError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelationDefinition {
    pub id: String,
    pub source_type: String,
    pub target_type: String,
    pub name: String,
    pub inverse_name: String,
    pub built_in: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RelationDefinitionInput {
    pub id: Option<String>,
    pub source_type: String,
    pub target_type: String,
    pub name: String,
    pub inverse_name: String,
}

const BUILT_INS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "builtin_lab_university",
        "lab",
        "university",
        "Institutions",
        "Labs",
    ),
    ("builtin_person_lab", "person", "lab", "Labs", "People"),
    (
        "builtin_program_university",
        "program",
        "university",
        "University",
        "Programs",
    ),
    (
        "builtin_application_program",
        "application",
        "program",
        "Program",
        "Applications",
    ),
    (
        "builtin_application_person",
        "application",
        "person",
        "Contacts",
        "Applications",
    ),
    (
        "builtin_publication_person",
        "publication",
        "person",
        "Authors",
        "Publications",
    ),
    ("builtin_project_lab", "project", "lab", "Labs", "Projects"),
];

impl Db {
    pub(super) fn seed_relation_definitions(&mut self) -> Result<()> {
        let tx = self.conn.transaction()?;
        for (id, source, target, name, inverse_name) in BUILT_INS {
            tx.execute(
                "INSERT OR IGNORE INTO relation_definitions
                 (id, source_type, target_type, name, inverse_name, built_in)
                 VALUES(?1, ?2, ?3, ?4, ?5, 1)",
                params![id, source, target, name, inverse_name],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_relation_definitions(
        &self,
        type_key: Option<&str>,
    ) -> Result<Vec<RelationDefinition>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_type, target_type, name, inverse_name, built_in
             FROM relation_definitions
             WHERE (?1 IS NULL OR source_type = ?1 OR target_type = ?1)
             ORDER BY built_in DESC, name COLLATE NOCASE, id",
        )?;
        let rows = stmt.query_map([type_key], |row| {
            Ok(RelationDefinition {
                id: row.get(0)?,
                source_type: row.get(1)?,
                target_type: row.get(2)?,
                name: row.get(3)?,
                inverse_name: row.get(4)?,
                built_in: row.get(5)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn get_relation_definition(&self, id: &str) -> Result<Option<RelationDefinition>> {
        self.conn
            .query_row(
                "SELECT id, source_type, target_type, name, inverse_name, built_in
             FROM relation_definitions WHERE id = ?1",
                [id],
                |row| {
                    Ok(RelationDefinition {
                        id: row.get(0)?,
                        source_type: row.get(1)?,
                        target_type: row.get(2)?,
                        name: row.get(3)?,
                        inverse_name: row.get(4)?,
                        built_in: row.get(5)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn put_relation_definition(
        &mut self,
        input: RelationDefinitionInput,
    ) -> Result<RelationDefinition> {
        if input.name.trim().is_empty()
            || input.inverse_name.trim().is_empty()
            || input.name.len() > 100
            || input.inverse_name.len() > 100
        {
            return Err(DbError::Invalid(
                "relationship labels must be 1-100 characters".into(),
            ));
        }
        let known = self.list_types()?;
        if !known.iter().any(|kind| kind.key == input.source_type)
            || !known.iter().any(|kind| kind.key == input.target_type)
        {
            return Err(DbError::Invalid(
                "relationship refers to an unknown entity type".into(),
            ));
        }
        let id = match input.id {
            Some(id) => {
                let existing = self
                    .get_relation_definition(&id)?
                    .ok_or(DbError::NotFound)?;
                if existing.built_in
                    || existing.source_type != input.source_type
                    || existing.target_type != input.target_type
                {
                    return Err(DbError::Invalid(
                        "built-in relationships and endpoints cannot be changed".into(),
                    ));
                }
                self.conn.execute(
                    "UPDATE relation_definitions SET name = ?2, inverse_name = ?3 WHERE id = ?1",
                    params![id, input.name.trim(), input.inverse_name.trim()],
                )?;
                id
            }
            None => {
                let id = Uuid::new_v4().to_string();
                self.conn.execute(
                    "INSERT INTO relation_definitions(id, source_type, target_type, name, inverse_name, built_in)
                     VALUES(?1, ?2, ?3, ?4, ?5, 0)",
                    params![id, input.source_type, input.target_type, input.name.trim(), input.inverse_name.trim()],
                )?;
                id
            }
        };
        self.get_relation_definition(&id)?.ok_or(DbError::NotFound)
    }
}
