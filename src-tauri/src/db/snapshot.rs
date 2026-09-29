use base64::Engine;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{
    AgentAction, ChatEvent, ChatMessage, ChatSession, Db, DbError, Record, RecordType, Relation,
    RelationDefinition, Result, TableView, json_sql_error, read_agent_action, safe_managed_path,
    unix_seconds,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonBundleFile {
    pub path: String,
    pub sha256: String,
    pub content_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonBundleSetting {
    pub key: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonBundleExternalMessage {
    pub provider: String,
    pub account: String,
    pub external_id: String,
    pub record_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonBundle {
    pub format: String,
    pub version: u32,
    pub exported_at: i64,
    pub types: Vec<RecordType>,
    pub records: Vec<Record>,
    pub relation_definitions: Vec<RelationDefinition>,
    pub relations: Vec<Relation>,
    pub views: Vec<TableView>,
    pub files: Vec<JsonBundleFile>,
    #[serde(default)]
    pub settings: Vec<JsonBundleSetting>,
    #[serde(default)]
    pub external_messages: Vec<JsonBundleExternalMessage>,
    #[serde(default)]
    pub chats: Vec<ChatSession>,
    #[serde(default)]
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub events: Vec<ChatEvent>,
    #[serde(default)]
    pub actions: Vec<AgentAction>,
}

impl Db {
    pub fn export_json_bundle(&self, include_history: bool) -> Result<JsonBundle> {
        let files = self
            .all_files()?
            .into_iter()
            .map(|(path, bytes)| JsonBundleFile {
                path,
                sha256: format!("{:x}", Sha256::digest(&bytes)),
                content_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
            })
            .collect();
        let mut settings_stmt = self
            .conn
            .prepare("SELECT key, value_json FROM app_settings ORDER BY key")?;
        let settings = settings_stmt
            .query_map([], |row| {
                let encoded: String = row.get(1)?;
                Ok(JsonBundleSetting {
                    key: row.get(0)?,
                    value: serde_json::from_str(&encoded).map_err(json_sql_error)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut external_stmt = self.conn.prepare(
            "SELECT provider, account, external_id, record_id FROM external_messages
             ORDER BY provider, account, external_id",
        )?;
        let external_messages = external_stmt
            .query_map([], |row| {
                Ok(JsonBundleExternalMessage {
                    provider: row.get(0)?,
                    account: row.get(1)?,
                    external_id: row.get(2)?,
                    record_id: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let (chats, messages, events, actions) = if include_history {
            let mut chats = self.list_chat_sessions(false)?;
            chats.extend(self.list_chat_sessions(true)?);
            let mut messages = Vec::new();
            let mut events = Vec::new();
            for chat in &chats {
                messages.extend(self.list_chat_messages(&chat.id)?);
                events.extend(self.list_chat_events(&chat.id)?);
            }
            let mut actions_stmt = self.conn.prepare(
                "SELECT id, tool, arguments_json, target_types_json, status, created_at, error,
                        chat_message_id, decision, decided_at FROM agent_actions ORDER BY id",
            )?;
            let actions = actions_stmt
                .query_map([], read_agent_action)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            (chats, messages, events, actions)
        } else {
            (Vec::new(), Vec::new(), Vec::new(), Vec::new())
        };
        Ok(JsonBundle {
            format: "eduport-json".into(),
            version: 1,
            exported_at: unix_seconds(),
            types: self.list_types()?,
            records: self.list_records(None, true)?,
            relation_definitions: self.list_relation_definitions(None)?,
            relations: self.all_relations()?,
            views: self.all_custom_views()?,
            files,
            settings,
            external_messages,
            chats,
            messages,
            events,
            actions,
        })
    }

    pub fn import_json_bundle(&mut self, bundle: &JsonBundle) -> Result<()> {
        if bundle.format != "eduport-json" || bundle.version != 1 {
            return Err(DbError::Invalid("unsupported Eduport JSON backup".into()));
        }
        let files = bundle
            .files
            .iter()
            .map(|file| {
                if !safe_managed_path(&file.path) {
                    return Err(DbError::Invalid(format!("unsafe file path: {}", file.path)));
                }
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(&file.content_base64)
                    .map_err(|error| DbError::Invalid(format!("invalid file base64: {error}")))?;
                if format!("{:x}", Sha256::digest(&bytes)) != file.sha256 {
                    return Err(DbError::Invalid(format!(
                        "file checksum mismatch: {}",
                        file.path
                    )));
                }
                Ok((file.path.clone(), bytes))
            })
            .collect::<Result<Vec<_>>>()?;
        self.conn.execute_batch("SAVEPOINT eduport_json_import")?;
        let result = (|| {
            self.import_snapshot(
                &bundle.types,
                &bundle.records,
                &bundle.relation_definitions,
                &bundle.relations,
                &bundle.views,
                &files,
            )?;
            for setting in &bundle.settings {
                if let Some(existing) = self.get_setting(&setting.key)? {
                    if existing != setting.value {
                        return Err(DbError::Invalid(format!(
                            "setting {} conflicts with existing data",
                            setting.key
                        )));
                    }
                } else {
                    self.put_setting(&setting.key, &setting.value)?;
                }
            }
            for reference in &bundle.external_messages {
                self.conn.execute(
                    "INSERT INTO external_messages(provider, account, external_id, record_id)
                     VALUES(?1, ?2, ?3, ?4)",
                    params![
                        reference.provider,
                        reference.account,
                        reference.external_id,
                        reference.record_id
                    ],
                )?;
            }
            for chat in &bundle.chats {
                self.conn.execute(
                    "INSERT INTO chat_sessions(id, title, created_at, updated_at, archived_at)
                     VALUES(?1, ?2, ?3, ?4, ?5)",
                    params![
                        chat.id,
                        chat.title,
                        chat.created_at,
                        chat.updated_at,
                        chat.archived_at
                    ],
                )?;
            }
            let mut messages = bundle.messages.iter().collect::<Vec<_>>();
            messages.sort_by_key(|message| (&message.session_id, message.sequence));
            for message in messages {
                self.conn.execute(
                    "INSERT INTO chat_messages(id, session_id, sequence, reply_to, role, content,
                                               status, provider, model, error, created_at, updated_at)
                     VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        message.id,
                        message.session_id,
                        message.sequence,
                        message.reply_to,
                        message.role,
                        message.content,
                        message.status,
                        message.provider,
                        message.model,
                        message.error,
                        message.created_at,
                        message.updated_at
                    ],
                )?;
            }
            for action in &bundle.actions {
                self.conn.execute(
                    "INSERT INTO agent_actions(id, tool, arguments_json, target_types_json, status,
                                               created_at, error, chat_message_id, decision, decided_at)
                     VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        action.id,
                        action.tool,
                        action.arguments.to_string(),
                        serde_json::to_string(&action.target_types).unwrap(),
                        action.status,
                        action.created_at,
                        action.error,
                        action.chat_message_id,
                        action.decision,
                        action.decided_at
                    ],
                )?;
            }
            for event in &bundle.events {
                self.conn.execute(
                    "INSERT INTO chat_events(id, assistant_message_id, ordinal, tool,
                                             arguments_json, result_json, action_id, created_at)
                     VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        event.id,
                        event.assistant_message_id,
                        event.ordinal,
                        event.tool,
                        event.arguments.to_string(),
                        event.result.to_string(),
                        event.action_id,
                        event.created_at
                    ],
                )?;
            }
            Ok(())
        })();
        if result.is_ok() {
            self.conn.execute_batch("RELEASE eduport_json_import")?;
        } else {
            self.conn
                .execute_batch("ROLLBACK TO eduport_json_import; RELEASE eduport_json_import")?;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::RecordInput;
    use serde_json::json;

    #[test]
    fn json_backup_round_trips_history_and_rolls_back_on_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let mut source = Db::open(&dir.path().join("source.sqlite")).unwrap();
        source
            .put_record(RecordInput {
                id: None,
                type_key: "note".into(),
                title: "Source note".into(),
                body: "Keep this".into(),
                values: json!({}),
                tags: vec!["backup".into()],
                expected_revision: None,
            })
            .unwrap();
        source.conn.execute(
            "INSERT INTO managed_files(relative_path, content) VALUES('source/original.md', ?1)",
            [b"original".as_slice()],
        ).unwrap();
        source
            .put_setting("backup-test", &json!({"enabled":true}))
            .unwrap();
        let chat = source.create_chat_session().unwrap();
        let (_, answer) = source
            .begin_chat_turn(&chat.id, "Question", "ollama", "mock")
            .unwrap();
        let action = source
            .queue_chat_agent_action(
                "create_record",
                &json!({"type_key":"note","title":"Proposed"}),
                &["note".into()],
                Some(&answer.id),
            )
            .unwrap();
        source
            .append_chat_event(
                &answer.id,
                1,
                "create_record",
                &action.arguments,
                &json!({"status":"pending"}),
                Some(&action.id),
            )
            .unwrap();
        source
            .finish_chat_message(&answer.id, "Waiting", None)
            .unwrap();

        let full = source.export_json_bundle(true).unwrap();
        assert_eq!(full.records.len(), 1);
        assert_eq!(full.chats.len(), 1);
        assert_eq!(full.actions.len(), 1);
        assert_eq!(source.export_json_bundle(false).unwrap().chats.len(), 0);
        let serialized = serde_json::to_vec(&full).unwrap();
        let parsed: JsonBundle = serde_json::from_slice(&serialized).unwrap();
        let mut restored = Db::open(&dir.path().join("restored.sqlite")).unwrap();
        restored.import_json_bundle(&parsed).unwrap();
        assert_eq!(restored.list_records(None, false).unwrap().len(), 1);
        assert_eq!(restored.list_chat_messages(&chat.id).unwrap().len(), 2);
        assert_eq!(restored.list_chat_events(&chat.id).unwrap().len(), 1);
        assert_eq!(restored.list_chat_actions(&chat.id).unwrap().len(), 1);
        assert_eq!(restored.all_files().unwrap()[0].1, b"original");
        assert_eq!(
            restored.get_setting("backup-test").unwrap(),
            Some(json!({"enabled":true}))
        );
        assert!(restored.import_json_bundle(&parsed).is_err());
        assert_eq!(restored.list_records(None, false).unwrap().len(), 1);

        let mut conflicted = Db::open(&dir.path().join("conflicted.sqlite")).unwrap();
        conflicted
            .put_setting("backup-test", &json!({"enabled":false}))
            .unwrap();
        assert!(conflicted.import_json_bundle(&parsed).is_err());
        assert!(conflicted.list_records(None, false).unwrap().is_empty());
        assert!(conflicted.list_chat_sessions(false).unwrap().is_empty());
    }
}
