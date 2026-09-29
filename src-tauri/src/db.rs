use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

mod chats;
mod relations_schema;
mod snapshot;
mod views;
pub use chats::{ChatEvent, ChatMessage, ChatSession};
pub use relations_schema::{RelationDefinition, RelationDefinitionInput};
pub use snapshot::{JsonBundle, JsonBundleFile};
pub use views::{FilterGroup, FilterRule, SortRule, TableView, TableViewInput, ViewConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAction {
    pub id: String,
    pub tool: String,
    pub arguments: Value,
    pub target_types: Vec<String>,
    pub status: String,
    pub created_at: i64,
    pub error: Option<String>,
    pub chat_message_id: Option<String>,
    pub decision: Option<String>,
    pub decided_at: Option<i64>,
}

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("record changed since it was opened")]
    Conflict,
    #[error("record not found")]
    NotFound,
}

pub type Result<T> = std::result::Result<T, DbError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    pub key: String,
    pub label: String,
    pub kind: String,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordType {
    pub key: String,
    pub label: String,
    pub icon: String,
    pub fields: Vec<FieldDef>,
    pub built_in: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub type_key: String,
    pub title: String,
    pub body: String,
    pub values: Value,
    pub tags: Vec<String>,
    pub revision: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecordInput {
    pub id: Option<String>,
    pub type_key: String,
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default = "empty_object")]
    pub values: Value,
    #[serde(default)]
    pub tags: Vec<String>,
    pub expected_revision: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub kind: String,
    pub role: Option<String>,
    pub position: Option<i64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    #[serde(default)]
    pub definition_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RelationInput {
    pub source_id: String,
    pub target_id: String,
    pub kind: String,
    pub role: Option<String>,
    pub position: Option<i64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    #[serde(default)]
    pub definition_id: Option<String>,
}

pub struct ExternalMail {
    pub external_id: String,
    pub title: String,
    pub body: String,
    pub from: String,
    pub to: String,
    pub date: String,
    pub raw: Vec<u8>,
}

fn empty_object() -> Value {
    Value::Object(Map::new())
}

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DbError::Invalid(format!("create app data: {e}")))?;
        }
        let conn = Connection::open(path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = FULL;",
        )?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > 6 {
            return Err(DbError::Invalid(format!(
                "database schema version {version} is newer than this app"
            )));
        }
        if version == 0 {
            conn.execute_batch(
                "BEGIN;
                 CREATE TABLE record_types (
                   key TEXT PRIMARY KEY,
                   label TEXT NOT NULL,
                   icon TEXT NOT NULL,
                   fields_json TEXT NOT NULL CHECK(json_valid(fields_json)),
                   built_in INTEGER NOT NULL
                 );
                 CREATE TABLE records (
                   id TEXT PRIMARY KEY,
                   type_key TEXT NOT NULL REFERENCES record_types(key),
                   title TEXT NOT NULL,
                   body TEXT NOT NULL DEFAULT '',
                   values_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(values_json)),
                   tags_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(tags_json)),
                   revision INTEGER NOT NULL DEFAULT 1,
                   created_at INTEGER NOT NULL,
                   updated_at INTEGER NOT NULL,
                   deleted_at INTEGER
                 );
                 CREATE INDEX idx_records_type ON records(type_key, deleted_at, updated_at);
                 CREATE TABLE relations (
                   id TEXT PRIMARY KEY,
                   source_id TEXT NOT NULL REFERENCES records(id) ON DELETE CASCADE,
                   target_id TEXT NOT NULL REFERENCES records(id) ON DELETE CASCADE,
                   kind TEXT NOT NULL,
                   role TEXT,
                   position INTEGER,
                   start_date TEXT,
                   end_date TEXT,
                   CHECK(source_id != target_id)
                 );
                 CREATE INDEX idx_relations_source ON relations(source_id);
                 CREATE INDEX idx_relations_target ON relations(target_id);
                 CREATE VIRTUAL TABLE records_fts USING fts5(
                   id UNINDEXED, title, body, values_json
                 );
                 CREATE TRIGGER records_fts_insert AFTER INSERT ON records
                 WHEN new.deleted_at IS NULL BEGIN
                   INSERT INTO records_fts(id, title, body, values_json)
                   VALUES(new.id, new.title, new.body, new.values_json);
                 END;
                 CREATE TRIGGER records_fts_update AFTER UPDATE ON records BEGIN
                   DELETE FROM records_fts WHERE id = old.id;
                   INSERT INTO records_fts(id, title, body, values_json)
                   SELECT new.id, new.title, new.body, new.values_json
                   WHERE new.deleted_at IS NULL;
                 END;
                 CREATE TRIGGER records_fts_delete AFTER DELETE ON records BEGIN
                   DELETE FROM records_fts WHERE id = old.id;
                 END;
                 CREATE TABLE app_settings (
                   key TEXT PRIMARY KEY,
                   value_json TEXT NOT NULL CHECK(json_valid(value_json))
                 );
                 CREATE TABLE managed_files (
                   relative_path TEXT PRIMARY KEY,
                   content BLOB NOT NULL
                 );
                 PRAGMA user_version = 1;
                 COMMIT;",
            )?;
        }
        if version < 2 {
            conn.execute_batch(
                "BEGIN;
                 CREATE TABLE external_messages (
                   provider TEXT NOT NULL,
                   account TEXT NOT NULL,
                   external_id TEXT NOT NULL,
                   record_id TEXT NOT NULL UNIQUE REFERENCES records(id) ON DELETE CASCADE,
                   PRIMARY KEY(provider, account, external_id)
                 );
                 PRAGMA user_version = 2;
                 COMMIT;",
            )?;
        }
        let mut db = Self { conn };
        db.seed_types()?;
        if version < 3 {
            db.upgrade_builtin_fields()?;
        }
        if version < 4 {
            db.upgrade_schema_v4()?;
        }
        db.seed_relation_definitions()?;
        if version < 5 {
            db.upgrade_schema_v5()?;
        }
        if version < 6 {
            db.upgrade_schema_v6()?;
        }
        Ok(db)
    }

    fn upgrade_schema_v5(&mut self) -> Result<()> {
        self.conn.execute_batch(
            "BEGIN;
             CREATE TABLE agent_actions (
               id TEXT PRIMARY KEY,
               tool TEXT NOT NULL,
               arguments_json TEXT NOT NULL CHECK(json_valid(arguments_json)),
               target_types_json TEXT NOT NULL CHECK(json_valid(target_types_json)),
               status TEXT NOT NULL CHECK(status IN ('pending', 'applying', 'applied', 'rejected')),
               created_at INTEGER NOT NULL,
               error TEXT
             );
             CREATE INDEX idx_agent_actions_status ON agent_actions(status, created_at DESC);
             PRAGMA user_version = 5;
             COMMIT;",
        )?;
        Ok(())
    }

    fn upgrade_schema_v6(&mut self) -> Result<()> {
        self.conn.execute_batch(
            "BEGIN;
             CREATE TABLE chat_sessions (
               id TEXT PRIMARY KEY,
               title TEXT NOT NULL,
               created_at INTEGER NOT NULL,
               updated_at INTEGER NOT NULL,
               archived_at INTEGER
             );
             CREATE INDEX idx_chat_sessions_updated ON chat_sessions(archived_at, updated_at DESC);
             CREATE TABLE chat_messages (
               id TEXT PRIMARY KEY,
               session_id TEXT NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
               sequence INTEGER NOT NULL,
               reply_to TEXT REFERENCES chat_messages(id),
               role TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
               content TEXT NOT NULL DEFAULT '',
               status TEXT NOT NULL CHECK(status IN ('running', 'completed', 'failed')),
               provider TEXT,
               model TEXT,
               error TEXT,
               created_at INTEGER NOT NULL,
               updated_at INTEGER NOT NULL,
               UNIQUE(session_id, sequence)
             );
             CREATE INDEX idx_chat_messages_session ON chat_messages(session_id, sequence);
             CREATE TABLE chat_events (
               id TEXT PRIMARY KEY,
               assistant_message_id TEXT NOT NULL REFERENCES chat_messages(id) ON DELETE CASCADE,
               ordinal INTEGER NOT NULL,
               tool TEXT NOT NULL,
               arguments_json TEXT NOT NULL CHECK(json_valid(arguments_json)),
               result_json TEXT NOT NULL CHECK(json_valid(result_json)),
               action_id TEXT REFERENCES agent_actions(id),
               created_at INTEGER NOT NULL,
               UNIQUE(assistant_message_id, ordinal)
             );
             ALTER TABLE agent_actions ADD COLUMN chat_message_id TEXT REFERENCES chat_messages(id);
             ALTER TABLE agent_actions ADD COLUMN decision TEXT CHECK(decision IN ('auto', 'approved', 'rejected'));
             ALTER TABLE agent_actions ADD COLUMN decided_at INTEGER;
             PRAGMA user_version = 6;
             COMMIT;",
        )?;
        Ok(())
    }

    fn upgrade_schema_v4(&mut self) -> Result<()> {
        self.conn.execute_batch(
            "BEGIN;
             CREATE TABLE relation_definitions (
               id TEXT PRIMARY KEY,
               source_type TEXT NOT NULL REFERENCES record_types(key),
               target_type TEXT NOT NULL REFERENCES record_types(key),
               name TEXT NOT NULL,
               inverse_name TEXT NOT NULL,
               built_in INTEGER NOT NULL DEFAULT 0,
               UNIQUE(source_type, target_type, name)
             );
             CREATE TABLE table_views (
               id TEXT PRIMARY KEY,
               type_key TEXT NOT NULL REFERENCES record_types(key),
               name TEXT NOT NULL,
               config_json TEXT NOT NULL CHECK(json_valid(config_json)),
               UNIQUE(type_key, name)
             );
             ALTER TABLE relations ADD COLUMN definition_id TEXT REFERENCES relation_definitions(id);
             PRAGMA user_version = 4;
             COMMIT;",
        )?;
        Ok(())
    }

    fn seed_types(&mut self) -> Result<()> {
        let tx = self.conn.transaction()?;
        for (key, label, icon, fields) in built_in_types() {
            tx.execute(
                "INSERT OR IGNORE INTO record_types(key, label, icon, fields_json, built_in)
                 VALUES(?1, ?2, ?3, ?4, 1)",
                params![key, label, icon, serde_json::to_string(&fields).unwrap()],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    fn upgrade_builtin_fields(&mut self) -> Result<()> {
        let tx = self.conn.transaction()?;
        for (key, _, _, defaults) in built_in_types() {
            let stored: String = tx.query_row(
                "SELECT fields_json FROM record_types WHERE key = ?1",
                [key],
                |row| row.get(0),
            )?;
            let mut fields: Vec<FieldDef> = serde_json::from_str(&stored)
                .map_err(|error| DbError::Invalid(error.to_string()))?;
            let mut ordered = Vec::new();
            for default in defaults {
                if let Some(index) = fields.iter().position(|field| field.key == default.key) {
                    let mut existing = fields.remove(index);
                    if default.required {
                        existing.required = true;
                    }
                    if existing.kind == "select" {
                        for option in default.options {
                            if !existing.options.contains(&option) {
                                existing.options.push(option);
                            }
                        }
                    }
                    ordered.push(existing);
                } else {
                    ordered.push(default);
                }
            }
            ordered.extend(fields);
            tx.execute(
                "UPDATE record_types SET fields_json = ?2 WHERE key = ?1",
                params![key, serde_json::to_string(&ordered).unwrap()],
            )?;
        }
        tx.pragma_update(None, "user_version", 3)?;
        tx.commit()?;
        Ok(())
    }

    pub fn list_types(&self) -> Result<Vec<RecordType>> {
        let mut stmt = self.conn.prepare(
            "SELECT key, label, icon, fields_json, built_in
             FROM record_types ORDER BY built_in DESC, label COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], |row| {
            let fields_text: String = row.get(3)?;
            Ok(RecordType {
                key: row.get(0)?,
                label: row.get(1)?,
                icon: row.get(2)?,
                fields: serde_json::from_str(&fields_text).map_err(json_sql_error)?,
                built_in: row.get(4)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn put_type(&mut self, definition: RecordType) -> Result<RecordType> {
        validate_type(&definition)?;
        let existing: Option<bool> = self
            .conn
            .query_row(
                "SELECT built_in FROM record_types WHERE key = ?1",
                [&definition.key],
                |row| row.get(0),
            )
            .optional()?;
        if existing == Some(true) && !definition.built_in {
            return Err(DbError::Invalid("cannot replace a built-in type".into()));
        }
        self.conn.execute(
            "INSERT INTO record_types(key, label, icon, fields_json, built_in)
             VALUES(?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(key) DO UPDATE SET
               label = excluded.label,
               icon = excluded.icon,
               fields_json = excluded.fields_json",
            params![
                definition.key,
                definition.label,
                definition.icon,
                serde_json::to_string(&definition.fields).unwrap(),
                definition.built_in
            ],
        )?;
        Ok(definition)
    }

    pub fn list_records(
        &self,
        type_key: Option<&str>,
        include_deleted: bool,
    ) -> Result<Vec<Record>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, type_key, title, body, values_json, revision,
                    created_at, updated_at, deleted_at, tags_json
             FROM records
             WHERE (?1 IS NULL OR type_key = ?1)
               AND (?2 OR deleted_at IS NULL)
             ORDER BY updated_at DESC, title COLLATE NOCASE",
        )?;
        let rows = stmt.query_map(params![type_key, include_deleted], read_record)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn get_record(&self, id: &str) -> Result<Option<Record>> {
        self.conn
            .query_row(
                "SELECT id, type_key, title, body, values_json, revision,
                        created_at, updated_at, deleted_at, tags_json
                 FROM records WHERE id = ?1",
                [id],
                read_record,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn put_record(&mut self, input: RecordInput) -> Result<Record> {
        let kind = self
            .list_types()?
            .into_iter()
            .find(|item| item.key == input.type_key)
            .ok_or_else(|| DbError::Invalid(format!("unknown type: {}", input.type_key)))?;
        validate_record(&input, &kind, true)?;
        let now = unix_seconds();
        let tx = self.conn.transaction()?;
        let id = if let Some(id) = input.id {
            let expected = input.expected_revision.ok_or(DbError::Conflict)?;
            let changed = tx.execute(
                "UPDATE records
                 SET title = ?2, body = ?3, values_json = ?4, tags_json = ?5,
                     revision = revision + 1, updated_at = ?6
                 WHERE id = ?1 AND type_key = ?7 AND revision = ?8 AND deleted_at IS NULL",
                params![
                    id,
                    input.title,
                    input.body,
                    input.values.to_string(),
                    serde_json::to_string(&input.tags).unwrap(),
                    now,
                    input.type_key,
                    expected
                ],
            )?;
            if changed != 1 {
                return Err(DbError::Conflict);
            }
            id
        } else {
            let id = Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO records(id, type_key, title, body, values_json, tags_json, created_at, updated_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![
                    id,
                    input.type_key,
                    input.title,
                    input.body,
                    input.values.to_string(),
                    serde_json::to_string(&input.tags).unwrap(),
                    now
                ],
            )?;
            id
        };
        tx.commit()?;
        self.get_record(&id)?.ok_or(DbError::NotFound)
    }

    pub fn set_deleted(&mut self, id: &str, deleted: bool) -> Result<Record> {
        let changed = if deleted {
            self.conn.execute(
                "UPDATE records SET deleted_at = ?2, revision = revision + 1, updated_at = ?2
                 WHERE id = ?1 AND deleted_at IS NULL",
                params![id, unix_seconds()],
            )?
        } else {
            self.conn.execute(
                "UPDATE records SET deleted_at = NULL, revision = revision + 1, updated_at = ?2
                 WHERE id = ?1 AND deleted_at IS NOT NULL",
                params![id, unix_seconds()],
            )?
        };
        if changed != 1 {
            return Err(DbError::NotFound);
        }
        self.get_record(id)?.ok_or(DbError::NotFound)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Record>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        let mut stmt = self.conn.prepare(
            "SELECT r.id, r.type_key, r.title, r.body, r.values_json, r.revision,
                    r.created_at, r.updated_at, r.deleted_at, r.tags_json
             FROM records_fts f
             JOIN records r ON r.id = f.id
             WHERE records_fts MATCH ?1 AND r.deleted_at IS NULL
             ORDER BY rank LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![query, limit.min(100) as i64], read_record)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn list_relations(&self, record_id: &str) -> Result<Vec<Relation>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_id, target_id, kind, role, position, start_date, end_date, definition_id
             FROM relations WHERE source_id = ?1 OR target_id = ?1
             ORDER BY kind, position, id",
        )?;
        let rows = stmt.query_map([record_id], |row| {
            Ok(Relation {
                id: row.get(0)?,
                source_id: row.get(1)?,
                target_id: row.get(2)?,
                kind: row.get(3)?,
                role: row.get(4)?,
                position: row.get(5)?,
                start_date: row.get(6)?,
                end_date: row.get(7)?,
                definition_id: row.get(8)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn all_relations(&self) -> Result<Vec<Relation>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_id, target_id, kind, role, position, start_date, end_date, definition_id
             FROM relations ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Relation {
                id: row.get(0)?,
                source_id: row.get(1)?,
                target_id: row.get(2)?,
                kind: row.get(3)?,
                role: row.get(4)?,
                position: row.get(5)?,
                start_date: row.get(6)?,
                end_date: row.get(7)?,
                definition_id: row.get(8)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn get_relation(&self, id: &str) -> Result<Option<Relation>> {
        self.all_relations()
            .map(|relations| relations.into_iter().find(|relation| relation.id == id))
    }

    pub fn all_files(&self) -> Result<Vec<(String, Vec<u8>)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT relative_path, content FROM managed_files ORDER BY relative_path")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn list_file_metadata(&self, offset: usize, limit: usize) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT relative_path, length(content) FROM managed_files ORDER BY relative_path LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt.query_map(params![limit.min(100) as i64, offset as i64], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn read_file_chunk(
        &self,
        path: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Option<(Vec<u8>, i64)>> {
        self.conn.query_row(
            "SELECT substr(content, ?2, ?3), length(content) FROM managed_files WHERE relative_path = ?1",
            params![path, offset.saturating_add(1) as i64, limit.min(5000) as i64],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional().map_err(Into::into)
    }

    pub fn imported_message_ids(&self, provider: &str, account: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT external_id FROM external_messages WHERE provider = ?1 AND account = ?2",
        )?;
        let rows = stmt.query_map(params![provider, account], |row| row.get(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn import_external_mail(
        &mut self,
        account: &str,
        messages: &[ExternalMail],
    ) -> Result<usize> {
        if messages.is_empty() {
            return Ok(0);
        }
        let tx = self.conn.transaction()?;
        for message in messages {
            if message.external_id.is_empty() || message.title.trim().is_empty() {
                return Err(DbError::Invalid("mail id and subject are required".into()));
            }
            let id = Uuid::new_v4().to_string();
            let now = unix_seconds();
            let values = serde_json::json!({
                "from": message.from,
                "to": message.to,
                "date": message.date,
            });
            tx.execute(
                "INSERT INTO records(id, type_key, title, body, values_json, tags_json, created_at, updated_at)
                 VALUES(?1, 'mail', ?2, ?3, ?4, '[]', ?5, ?5)",
                params![id, message.title, message.body, values.to_string(), now],
            )?;
            tx.execute(
                "INSERT INTO external_messages(provider, account, external_id, record_id)
                 VALUES('gmail', ?1, ?2, ?3)",
                params![account, message.external_id, id],
            )?;
            tx.execute(
                "INSERT INTO managed_files(relative_path, content) VALUES(?1, ?2)",
                params![format!("mail-original/{id}.eml"), message.raw],
            )?;
        }
        tx.commit()?;
        Ok(messages.len())
    }

    pub fn import_snapshot(
        &mut self,
        types: &[RecordType],
        records: &[Record],
        relation_definitions: &[RelationDefinition],
        relations: &[Relation],
        views: &[TableView],
        files: &[(String, Vec<u8>)],
    ) -> Result<()> {
        let mut merged_types = self.list_types()?;
        for definition in types {
            validate_type(definition)?;
            if let Some(current) = merged_types
                .iter_mut()
                .find(|item| item.key == definition.key)
            {
                for field in &definition.fields {
                    if let Some(existing) =
                        current.fields.iter_mut().find(|item| item.key == field.key)
                    {
                        if existing.kind != field.kind {
                            return Err(DbError::Invalid(format!(
                                "field {} has incompatible types in {}",
                                field.key, definition.key
                            )));
                        }
                        for option in &field.options {
                            if !existing.options.contains(option) {
                                existing.options.push(option.clone());
                            }
                        }
                    } else {
                        current.fields.push(field.clone());
                    }
                }
            } else {
                merged_types.push(definition.clone());
            }
        }
        for definition in &merged_types {
            validate_type(definition)?;
        }
        for record in records {
            if !record
                .id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
            {
                return Err(DbError::Invalid(format!("unsafe record id: {}", record.id)));
            }
            if self.get_record(&record.id)?.is_some() {
                return Err(DbError::Invalid(format!(
                    "record {} already exists; no data was changed",
                    record.id
                )));
            }
            let definition = merged_types
                .iter()
                .find(|kind| kind.key == record.type_key)
                .ok_or_else(|| DbError::Invalid(format!("missing type {}", record.type_key)))?;
            validate_record(
                &RecordInput {
                    id: None,
                    type_key: record.type_key.clone(),
                    title: record.title.clone(),
                    body: record.body.clone(),
                    values: record.values.clone(),
                    tags: record.tags.clone(),
                    expected_revision: None,
                },
                definition,
                false,
            )?;
        }
        let mut merged_relation_definitions = self.list_relation_definitions(None)?;
        for definition in relation_definitions {
            if !merged_types
                .iter()
                .any(|kind| kind.key == definition.source_type)
                || !merged_types
                    .iter()
                    .any(|kind| kind.key == definition.target_type)
                || definition.name.trim().is_empty()
                || definition.inverse_name.trim().is_empty()
            {
                return Err(DbError::Invalid(format!(
                    "invalid relationship definition: {}",
                    definition.id
                )));
            }
            if let Some(existing) = self.get_relation_definition(&definition.id)?
                && existing != *definition
            {
                return Err(DbError::Invalid(format!(
                    "relationship definition {} conflicts",
                    definition.id
                )));
            }
            if !merged_relation_definitions
                .iter()
                .any(|item| item.id == definition.id)
            {
                merged_relation_definitions.push(definition.clone());
            }
        }
        for view in views {
            if view.is_default
                || self
                    .all_custom_views()?
                    .iter()
                    .any(|existing| existing.id == view.id)
            {
                return Err(DbError::Invalid(format!(
                    "view {} already exists or is a default view",
                    view.id
                )));
            }
            let kind = merged_types
                .iter()
                .find(|kind| kind.key == view.type_key)
                .ok_or_else(|| {
                    DbError::Invalid(format!("view {} has an unknown entity type", view.id))
                })?;
            views::validate_view(kind, &merged_relation_definitions, &view.name, &view.config)?;
        }
        for relation in relations {
            if let Some(definition_id) = &relation.definition_id {
                let definition = relation_definitions
                    .iter()
                    .find(|item| &item.id == definition_id)
                    .cloned()
                    .or(self.get_relation_definition(definition_id)?)
                    .ok_or_else(|| {
                        DbError::Invalid(format!(
                            "relationship {} has an unknown definition",
                            relation.id
                        ))
                    })?;
                let source = records
                    .iter()
                    .find(|item| item.id == relation.source_id)
                    .cloned()
                    .or(self.get_record(&relation.source_id)?)
                    .ok_or(DbError::NotFound)?;
                let target = records
                    .iter()
                    .find(|item| item.id == relation.target_id)
                    .cloned()
                    .or(self.get_record(&relation.target_id)?)
                    .ok_or(DbError::NotFound)?;
                if source.type_key != definition.source_type
                    || target.type_key != definition.target_type
                {
                    return Err(DbError::Invalid(format!(
                        "relationship {} has incompatible endpoints",
                        relation.id
                    )));
                }
            }
        }
        for (path, _) in files {
            if !safe_managed_path(path) {
                return Err(DbError::Invalid(format!("unsafe file path: {path}")));
            }
            let exists: bool = self.conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM managed_files WHERE relative_path = ?1)",
                [path],
                |row| row.get(0),
            )?;
            if exists {
                return Err(DbError::Invalid(format!(
                    "file {path} already exists; no data was changed"
                )));
            }
        }
        let tx = self.conn.savepoint()?;
        for definition in &merged_types {
            tx.execute(
                "INSERT INTO record_types(key, label, icon, fields_json, built_in)
                 VALUES(?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(key) DO UPDATE SET
                   label = excluded.label, icon = excluded.icon,
                   fields_json = excluded.fields_json",
                params![
                    definition.key,
                    definition.label,
                    definition.icon,
                    serde_json::to_string(&definition.fields).unwrap(),
                    definition.built_in,
                ],
            )?;
        }
        for definition in relation_definitions {
            tx.execute(
                "INSERT OR IGNORE INTO relation_definitions(id, source_type, target_type, name, inverse_name, built_in)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                params![definition.id, definition.source_type, definition.target_type,
                    definition.name, definition.inverse_name, definition.built_in],
            )?;
        }
        for view in views {
            tx.execute(
                "INSERT INTO table_views(id, type_key, name, config_json) VALUES(?1, ?2, ?3, ?4)",
                params![
                    view.id,
                    view.type_key,
                    view.name,
                    serde_json::to_string(&view.config).unwrap()
                ],
            )?;
        }
        for record in records {
            tx.execute(
                "INSERT INTO records(id, type_key, title, body, values_json, tags_json,
                                     revision, created_at, updated_at, deleted_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    record.id,
                    record.type_key,
                    record.title,
                    record.body,
                    record.values.to_string(),
                    serde_json::to_string(&record.tags).unwrap(),
                    record.revision,
                    record.created_at,
                    record.updated_at,
                    record.deleted_at,
                ],
            )?;
        }
        for relation in relations {
            tx.execute(
                "INSERT INTO relations(id, source_id, target_id, kind, role, position,
                                       start_date, end_date, definition_id)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    relation.id,
                    relation.source_id,
                    relation.target_id,
                    relation.kind,
                    relation.role,
                    relation.position,
                    relation.start_date,
                    relation.end_date,
                    relation.definition_id,
                ],
            )?;
        }
        for (path, bytes) in files {
            tx.execute(
                "INSERT INTO managed_files(relative_path, content) VALUES(?1, ?2)",
                params![path, bytes],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn put_relation(&mut self, input: RelationInput) -> Result<Relation> {
        if input.source_id == input.target_id {
            return Err(DbError::Invalid("invalid relation".into()));
        }
        let kind = if let Some(definition_id) = &input.definition_id {
            let definition = self
                .get_relation_definition(definition_id)?
                .ok_or_else(|| DbError::Invalid("unknown relationship definition".into()))?;
            let source = self
                .get_record(&input.source_id)?
                .ok_or(DbError::NotFound)?;
            let target = self
                .get_record(&input.target_id)?
                .ok_or(DbError::NotFound)?;
            if source.deleted_at.is_some()
                || target.deleted_at.is_some()
                || source.type_key != definition.source_type
                || target.type_key != definition.target_type
            {
                return Err(DbError::Invalid(
                    "relationship endpoints do not match their entity types".into(),
                ));
            }
            definition.name
        } else if input.kind.trim().is_empty() {
            return Err(DbError::Invalid("relationship kind is required".into()));
        } else {
            input.kind.clone()
        };
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO relations(id, source_id, target_id, kind, role, position, start_date, end_date, definition_id)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                input.source_id,
                input.target_id,
                kind,
                input.role,
                input.position,
                input.start_date,
                input.end_date,
                input.definition_id,
            ],
        )?;
        self.list_relations(&input.source_id)?
            .into_iter()
            .find(|relation| relation.id == id)
            .ok_or(DbError::NotFound)
    }

    pub fn delete_relation(&mut self, id: &str) -> Result<()> {
        if self
            .conn
            .execute("DELETE FROM relations WHERE id = ?1", [id])?
            != 1
        {
            return Err(DbError::NotFound);
        }
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<Value>> {
        let value: Option<String> = self
            .conn
            .query_row(
                "SELECT value_json FROM app_settings WHERE key = ?1",
                [key],
                |row| row.get(0),
            )
            .optional()?;
        value
            .map(|text| serde_json::from_str(&text).map_err(|e| DbError::Invalid(e.to_string())))
            .transpose()
    }

    pub fn put_setting(&mut self, key: &str, value: &Value) -> Result<()> {
        self.conn.execute(
            "INSERT INTO app_settings(key, value_json) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
            params![key, value.to_string()],
        )?;
        Ok(())
    }

    pub fn queue_agent_action(
        &mut self,
        tool: &str,
        arguments: &Value,
        target_types: &[String],
    ) -> Result<AgentAction> {
        self.queue_chat_agent_action(tool, arguments, target_types, None)
    }

    pub fn queue_chat_agent_action(
        &mut self,
        tool: &str,
        arguments: &Value,
        target_types: &[String],
        chat_message_id: Option<&str>,
    ) -> Result<AgentAction> {
        let id = Uuid::new_v4().to_string();
        let now = unix_seconds();
        self.conn.execute(
            "INSERT INTO agent_actions(id, tool, arguments_json, target_types_json, status, created_at, chat_message_id)
             VALUES(?1, ?2, ?3, ?4, 'pending', ?5, ?6)",
            params![id, tool, arguments.to_string(), serde_json::to_string(target_types).unwrap(), now, chat_message_id],
        )?;
        self.get_agent_action(&id)?.ok_or(DbError::NotFound)
    }

    pub fn get_agent_action(&self, id: &str) -> Result<Option<AgentAction>> {
        self.conn
            .query_row(
                "SELECT id, tool, arguments_json, target_types_json, status, created_at, error, chat_message_id, decision, decided_at
             FROM agent_actions WHERE id = ?1",
                [id],
                read_agent_action,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_agent_actions(&self) -> Result<Vec<AgentAction>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, tool, arguments_json, target_types_json, status, created_at, error, chat_message_id, decision, decided_at
             FROM agent_actions WHERE status IN ('pending', 'applying')
             ORDER BY created_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([], read_agent_action)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn transition_agent_action(
        &mut self,
        id: &str,
        from: &str,
        to: &str,
        error: Option<&str>,
    ) -> Result<()> {
        self.transition_agent_action_with_decision(id, from, to, error, None)
    }

    pub fn transition_agent_action_with_decision(
        &mut self,
        id: &str,
        from: &str,
        to: &str,
        error: Option<&str>,
        decision: Option<&str>,
    ) -> Result<()> {
        let changed = self.conn.execute(
            "UPDATE agent_actions
             SET status = ?3, error = ?4,
                 decision = COALESCE(?5, decision),
                 decided_at = CASE WHEN ?5 IS NOT NULL THEN ?6 ELSE decided_at END
             WHERE id = ?1 AND status = ?2",
            params![id, from, to, error, decision, unix_seconds()],
        )?;
        if changed != 1 {
            return Err(DbError::Conflict);
        }
        Ok(())
    }

    pub fn backup_to(&self, path: &Path) -> Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| DbError::Invalid("backup needs a parent folder".into()))?;
        std::fs::create_dir_all(parent)
            .map_err(|e| DbError::Invalid(format!("create backup folder: {e}")))?;
        let stage = tempfile::Builder::new()
            .prefix(".eduport-backup-")
            .tempdir_in(parent)
            .map_err(|e| DbError::Invalid(format!("stage backup: {e}")))?;
        let staged_path = stage.path().join("backup.sqlite");
        {
            let mut destination = Connection::open(&staged_path)?;
            let backup = rusqlite::backup::Backup::new(&self.conn, &mut destination)?;
            backup.run_to_completion(100, std::time::Duration::from_millis(10), None)?;
        }
        std::fs::hard_link(&staged_path, path).map_err(|e| {
            DbError::Invalid(format!(
                "save backup without overwriting existing files: {e}"
            ))
        })?;
        Ok(())
    }
}

fn read_agent_action(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentAction> {
    let arguments: String = row.get(2)?;
    let target_types: String = row.get(3)?;
    Ok(AgentAction {
        id: row.get(0)?,
        tool: row.get(1)?,
        arguments: serde_json::from_str(&arguments).map_err(json_sql_error)?,
        target_types: serde_json::from_str(&target_types).map_err(json_sql_error)?,
        status: row.get(4)?,
        created_at: row.get(5)?,
        error: row.get(6)?,
        chat_message_id: row.get(7)?,
        decision: row.get(8)?,
        decided_at: row.get(9)?,
    })
}

fn read_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<Record> {
    let values_text: String = row.get(4)?;
    let tags_text: String = row.get(9)?;
    Ok(Record {
        id: row.get(0)?,
        type_key: row.get(1)?,
        title: row.get(2)?,
        body: row.get(3)?,
        values: serde_json::from_str(&values_text).map_err(json_sql_error)?,
        tags: serde_json::from_str(&tags_text).map_err(json_sql_error)?,
        revision: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        deleted_at: row.get(8)?,
    })
}

fn json_sql_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

fn unix_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn safe_managed_path(path: &str) -> bool {
    let parsed = Path::new(path);
    !path.is_empty()
        && !parsed.is_absolute()
        && parsed
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}

fn validate_type(definition: &RecordType) -> Result<()> {
    if definition.key.is_empty()
        || !definition
            .key
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(DbError::Invalid(
            "type key must use a-z, 0-9 or underscore".into(),
        ));
    }
    if definition.label.trim().is_empty() {
        return Err(DbError::Invalid("type label is required".into()));
    }
    let mut seen = std::collections::HashSet::new();
    for field in &definition.fields {
        if field.key.is_empty()
            || !field
                .key
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
            || !seen.insert(&field.key)
        {
            return Err(DbError::Invalid(format!(
                "invalid or duplicate field: {}",
                field.key
            )));
        }
        if !matches!(
            field.kind.as_str(),
            "text" | "number" | "date" | "boolean" | "url" | "select"
        ) {
            return Err(DbError::Invalid(format!(
                "unknown field kind: {}",
                field.kind
            )));
        }
        if field.kind == "select" && field.options.is_empty() {
            return Err(DbError::Invalid(format!(
                "select {} needs options",
                field.key
            )));
        }
    }
    Ok(())
}

fn validate_record(input: &RecordInput, kind: &RecordType, enforce_required: bool) -> Result<()> {
    if input.title.trim().is_empty() {
        return Err(DbError::Invalid("record title is required".into()));
    }
    if input
        .tags
        .iter()
        .any(|tag| tag.trim().is_empty() || tag.len() > 80)
    {
        return Err(DbError::Invalid(
            "tags must be nonempty and at most 80 bytes".into(),
        ));
    }
    let values = input
        .values
        .as_object()
        .ok_or_else(|| DbError::Invalid("values must be an object".into()))?;
    for key in values.keys() {
        if !kind.fields.iter().any(|field| &field.key == key) {
            return Err(DbError::Invalid(format!("unknown field: {key}")));
        }
    }
    for field in &kind.fields {
        let value = values.get(&field.key).unwrap_or(&Value::Null);
        if enforce_required && field.required && (value.is_null() || value == "") {
            return Err(DbError::Invalid(format!("{} is required", field.label)));
        }
        if value.is_null() {
            continue;
        }
        let valid = match field.kind.as_str() {
            "text" | "date" | "url" => value.is_string(),
            "number" => value.is_number(),
            "boolean" => value.is_boolean(),
            "select" => value
                .as_str()
                .is_some_and(|selected| field.options.iter().any(|option| option == selected)),
            _ => false,
        };
        if !valid {
            return Err(DbError::Invalid(format!(
                "{} has the wrong value type",
                field.label
            )));
        }
    }
    Ok(())
}

fn field(key: &str, label: &str, kind: &str) -> FieldDef {
    FieldDef {
        key: key.into(),
        label: label.into(),
        kind: kind.into(),
        options: Vec::new(),
        required: false,
    }
}

fn select(key: &str, label: &str, options: &[&str]) -> FieldDef {
    FieldDef {
        options: options.iter().map(|option| option.to_string()).collect(),
        ..field(key, label, "select")
    }
}

fn required(mut definition: FieldDef) -> FieldDef {
    definition.required = true;
    definition
}

fn built_in_types() -> Vec<(&'static str, &'static str, &'static str, Vec<FieldDef>)> {
    vec![
        (
            "university",
            "Universities",
            "building",
            vec![
                field("country", "Country", "text"),
                field("city", "City", "text"),
                field("website", "Website", "url"),
            ],
        ),
        (
            "lab",
            "Labs",
            "flask",
            vec![
                field("focus", "Focus", "text"),
                field("website", "Website", "url"),
            ],
        ),
        (
            "person",
            "People",
            "user",
            vec![
                field("email", "Email", "text"),
                field("position", "Position", "text"),
                field("website", "Website", "url"),
            ],
        ),
        (
            "program",
            "Programs",
            "graduation",
            vec![
                required(select("level", "Level", &["masters", "phd"])),
                field("department", "Department", "text"),
                field("website", "Website", "url"),
            ],
        ),
        (
            "application",
            "Applications",
            "folder",
            vec![
                required(select(
                    "status",
                    "Status",
                    &[
                        "planning",
                        "drafting",
                        "submitted",
                        "decision_pending",
                        "accepted",
                        "rejected",
                        "withdrawn",
                    ],
                )),
                field("intake", "Intake", "text"),
                field("deadline", "Deadline", "date"),
            ],
        ),
        (
            "publication",
            "Publications",
            "book",
            vec![
                field("year", "Year", "number"),
                field("doi", "DOI", "text"),
                field("url", "URL", "url"),
            ],
        ),
        (
            "project",
            "Projects",
            "layers",
            vec![
                field("summary", "Summary", "text"),
                field("start_date", "Start date", "date"),
                field("end_date", "End date", "date"),
            ],
        ),
        (
            "mail",
            "Mail",
            "mail",
            vec![
                field("from", "From", "text"),
                field("to", "To", "text"),
                field("date", "Date", "date"),
            ],
        ),
        (
            "document",
            "Documents",
            "file",
            vec![
                select("status", "Status", &["requested", "drafting", "received"]),
                field("source_url", "Source URL", "url"),
            ],
        ),
        (
            "note",
            "Notes",
            "note",
            vec![field("source_url", "Source URL", "url")],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_are_validated_and_revisions_prevent_lost_updates() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Db::open(&dir.path().join("data.sqlite")).unwrap();
        let created = db
            .put_record(RecordInput {
                id: None,
                type_key: "program".into(),
                title: "MSc AI".into(),
                body: "Research track".into(),
                values: serde_json::json!({"level": "masters"}),
                tags: vec!["ai".into()],
                expected_revision: None,
            })
            .unwrap();
        assert_eq!(created.revision, 1);
        let updated = db
            .put_record(RecordInput {
                id: Some(created.id.clone()),
                type_key: "program".into(),
                title: "MSc AI".into(),
                body: "New text".into(),
                values: serde_json::json!({"level": "masters"}),
                tags: vec!["ai".into()],
                expected_revision: Some(1),
            })
            .unwrap();
        assert_eq!(updated.revision, 2);
        assert!(matches!(
            db.put_record(RecordInput {
                id: Some(created.id),
                type_key: "program".into(),
                title: "Stale".into(),
                body: String::new(),
                values: serde_json::json!({"level": "masters"}),
                tags: vec![],
                expected_revision: Some(1),
            }),
            Err(DbError::Conflict)
        ));
    }

    #[test]
    fn relations_require_existing_records_and_search_respects_trash() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Db::open(&dir.path().join("data.sqlite")).unwrap();
        let person = db
            .put_record(RecordInput {
                id: None,
                type_key: "person".into(),
                title: "Ada".into(),
                body: String::new(),
                values: empty_object(),
                tags: vec![],
                expected_revision: None,
            })
            .unwrap();
        let lab = db
            .put_record(RecordInput {
                id: None,
                type_key: "lab".into(),
                title: "Signal Lab".into(),
                body: "Speech research".into(),
                values: empty_object(),
                tags: vec![],
                expected_revision: None,
            })
            .unwrap();
        db.put_relation(RelationInput {
            source_id: person.id.clone(),
            target_id: lab.id.clone(),
            kind: "member_of".into(),
            role: Some("PI".into()),
            position: None,
            start_date: None,
            end_date: None,
            definition_id: None,
        })
        .unwrap();
        assert_eq!(db.list_relations(&lab.id).unwrap().len(), 1);
        assert_eq!(db.search("Speech", 10).unwrap().len(), 1);
        db.set_deleted(&lab.id, true).unwrap();
        assert!(db.search("Speech", 10).unwrap().is_empty());
    }

    #[test]
    fn gmail_import_is_atomic_and_keeps_original_mail() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Db::open(&dir.path().join("data.sqlite")).unwrap();
        let mail = |id: &str| ExternalMail {
            external_id: id.into(),
            title: format!("Subject {id}"),
            body: "Body".into(),
            from: "a@example.com".into(),
            to: "b@example.com".into(),
            date: "2026-09-28".into(),
            raw: b"Subject: Original\r\n\r\nBody".to_vec(),
        };
        assert_eq!(
            db.import_external_mail("me@example.com", &[mail("one")])
                .unwrap(),
            1
        );
        assert_eq!(db.all_files().unwrap().len(), 1);
        let (path, length) = db.list_file_metadata(0, 10).unwrap().remove(0);
        assert!(path.ends_with(".eml"));
        assert_eq!(
            db.read_file_chunk(&path, 0, 8).unwrap().unwrap().0,
            b"Subject:".to_vec()
        );
        assert!(length > 8);
        assert!(
            db.import_external_mail("me@example.com", &[mail("two"), mail("one")])
                .is_err()
        );
        assert_eq!(db.list_records(Some("mail"), false).unwrap().len(), 1);
        assert_eq!(
            db.imported_message_ids("gmail", "me@example.com").unwrap(),
            vec!["one"]
        );
    }

    #[test]
    fn backup_is_readable_and_never_overwrites_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Db::open(&dir.path().join("data.sqlite")).unwrap();
        db.put_record(RecordInput {
            id: None,
            type_key: "note".into(),
            title: "Recover me".into(),
            body: "A saved note".into(),
            values: empty_object(),
            tags: vec![],
            expected_revision: None,
        })
        .unwrap();
        let backup = dir.path().join("backup.sqlite");
        db.backup_to(&backup).unwrap();
        assert_eq!(
            Db::open(&backup)
                .unwrap()
                .list_records(None, false)
                .unwrap()
                .len(),
            1
        );
        assert!(db.backup_to(&backup).is_err());
        assert_eq!(
            Db::open(&backup)
                .unwrap()
                .list_records(None, false)
                .unwrap()[0]
                .title,
            "Recover me"
        );
    }

    #[test]
    fn built_in_fields_are_upgraded_without_losing_custom_fields_or_records() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.sqlite");
        let db = Db::open(&path).unwrap();
        let mut old_program = db
            .list_types()
            .unwrap()
            .into_iter()
            .find(|kind| kind.key == "program")
            .unwrap();
        old_program.fields = vec![
            select("level", "Level", &["masters", "phd"]),
            field("my_field", "My field", "text"),
        ];
        db.conn
            .execute(
                "UPDATE record_types SET fields_json = ?1 WHERE key = 'program'",
                [serde_json::to_string(&old_program.fields).unwrap()],
            )
            .unwrap();
        db.conn.execute(
            "INSERT INTO records(id, type_key, title, body, values_json, tags_json, created_at, updated_at)
             VALUES('legacy', 'program', 'Old program', '', '{}', '[]', 1, 1)",
            [],
        ).unwrap();
        db.conn
            .execute_batch(
                "DROP TABLE chat_events;
             DROP TABLE chat_messages;
             DROP TABLE chat_sessions;
             DROP TABLE agent_actions;
             ALTER TABLE relations DROP COLUMN definition_id;
             DROP TABLE relation_definitions;
             DROP TABLE table_views;",
            )
            .unwrap();
        db.conn.pragma_update(None, "user_version", 2).unwrap();
        drop(db);

        let mut upgraded = Db::open(&path).unwrap();
        let types = upgraded.list_types().unwrap();
        assert!(types.iter().all(|kind| !kind.fields.is_empty()));
        let program = types.iter().find(|kind| kind.key == "program").unwrap();
        assert!(program.fields.iter().any(|field| field.key == "my_field"));
        assert!(
            program
                .fields
                .iter()
                .find(|field| field.key == "level")
                .unwrap()
                .required
        );
        assert!(program.fields.iter().any(|field| field.key == "department"));
        assert_eq!(
            upgraded.get_record("legacy").unwrap().unwrap().title,
            "Old program"
        );
        assert!(
            upgraded
                .put_record(RecordInput {
                    id: None,
                    type_key: "program".into(),
                    title: "Incomplete".into(),
                    body: String::new(),
                    values: empty_object(),
                    tags: vec![],
                    expected_revision: None,
                })
                .is_err()
        );
    }

    #[test]
    fn custom_entity_views_and_relationships_validate_types() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Db::open(&dir.path().join("data.sqlite")).unwrap();
        db.put_type(RecordType {
            key: "scholarship".into(),
            label: "Scholarships".into(),
            icon: "file".into(),
            fields: vec![field("amount", "Amount", "number")],
            built_in: false,
        })
        .unwrap();
        assert_eq!(
            db.list_views("scholarship").unwrap()[0].config.columns,
            vec!["title", "amount", "tags", "updated_at"]
        );
        let saved = db
            .put_view(TableViewInput {
                id: None,
                type_key: "scholarship".into(),
                name: "Large awards".into(),
                config: ViewConfig {
                    columns: vec!["title".into(), "amount".into()],
                    filters: vec![FilterRule {
                        field: "amount".into(),
                        operator: "greater_than".into(),
                        value: "1000".into(),
                    }],
                    filter_tree: None,
                    sorts: vec![SortRule {
                        field: "amount".into(),
                        direction: "desc".into(),
                    }],
                    group_by: None,
                    column_widths: Default::default(),
                },
            })
            .unwrap();
        assert_eq!(db.list_views("scholarship").unwrap().len(), 2);
        let mut default = db.list_views("scholarship").unwrap()[0].clone();
        default.config.filter_tree = Some(FilterGroup {
            mode: "or".into(),
            rules: vec![FilterRule {
                field: "title".into(),
                operator: "contains".into(),
                value: "Award".into(),
            }],
            groups: vec![FilterGroup {
                mode: "and".into(),
                rules: vec![FilterRule {
                    field: "amount".into(),
                    operator: "greater_than".into(),
                    value: "1000".into(),
                }],
                groups: vec![],
            }],
        });
        db.put_view(TableViewInput {
            id: Some(default.id.clone()),
            type_key: "scholarship".into(),
            name: default.name,
            config: default.config,
        })
        .unwrap();
        assert!(
            db.list_views("scholarship").unwrap()[0]
                .config
                .filter_tree
                .is_some()
        );
        assert!(db.delete_view("default:scholarship").is_err());
        assert!(
            db.put_view(TableViewInput {
                id: Some(saved.id),
                type_key: "scholarship".into(),
                name: "Broken".into(),
                config: ViewConfig {
                    columns: vec!["missing".into()],
                    filters: vec![],
                    filter_tree: None,
                    sorts: vec![],
                    group_by: None,
                    column_widths: Default::default(),
                }
            })
            .is_err()
        );
        let definition = db
            .put_relation_definition(RelationDefinitionInput {
                id: None,
                source_type: "scholarship".into(),
                target_type: "university".into(),
                name: "Institution".into(),
                inverse_name: "Scholarships".into(),
            })
            .unwrap();
        let scholarship = db
            .put_record(RecordInput {
                id: None,
                type_key: "scholarship".into(),
                title: "Award".into(),
                body: String::new(),
                values: serde_json::json!({"amount": 2000}),
                tags: vec![],
                expected_revision: None,
            })
            .unwrap();
        let university = db
            .put_record(RecordInput {
                id: None,
                type_key: "university".into(),
                title: "University".into(),
                body: String::new(),
                values: empty_object(),
                tags: vec![],
                expected_revision: None,
            })
            .unwrap();
        let linked = db
            .put_relation(RelationInput {
                source_id: scholarship.id.clone(),
                target_id: university.id.clone(),
                kind: String::new(),
                role: None,
                position: None,
                start_date: None,
                end_date: None,
                definition_id: Some(definition.id.clone()),
            })
            .unwrap();
        assert_eq!(linked.definition_id, Some(definition.id.clone()));
        assert!(
            db.put_relation(RelationInput {
                source_id: university.id,
                target_id: scholarship.id,
                kind: String::new(),
                role: None,
                position: None,
                start_date: None,
                end_date: None,
                definition_id: Some(definition.id)
            })
            .is_err()
        );
    }
}
