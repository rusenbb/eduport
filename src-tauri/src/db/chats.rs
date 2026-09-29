use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::{AgentAction, Db, DbError, Result, json_sql_error, read_agent_action, unix_seconds};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub archived_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub session_id: String,
    pub sequence: i64,
    pub reply_to: Option<String>,
    pub role: String,
    pub content: String,
    pub status: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEvent {
    pub id: String,
    pub assistant_message_id: String,
    pub ordinal: i64,
    pub tool: String,
    pub arguments: Value,
    pub result: Value,
    pub action_id: Option<String>,
    pub created_at: i64,
}

fn read_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<ChatSession> {
    Ok(ChatSession {
        id: row.get(0)?,
        title: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get(3)?,
        archived_at: row.get(4)?,
    })
}

fn read_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<ChatMessage> {
    Ok(ChatMessage {
        id: row.get(0)?,
        session_id: row.get(1)?,
        sequence: row.get(2)?,
        reply_to: row.get(3)?,
        role: row.get(4)?,
        content: row.get(5)?,
        status: row.get(6)?,
        provider: row.get(7)?,
        model: row.get(8)?,
        error: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn read_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<ChatEvent> {
    let arguments: String = row.get(4)?;
    let result: String = row.get(5)?;
    Ok(ChatEvent {
        id: row.get(0)?,
        assistant_message_id: row.get(1)?,
        ordinal: row.get(2)?,
        tool: row.get(3)?,
        arguments: serde_json::from_str(&arguments).map_err(json_sql_error)?,
        result: serde_json::from_str(&result).map_err(json_sql_error)?,
        action_id: row.get(6)?,
        created_at: row.get(7)?,
    })
}

impl Db {
    pub fn create_chat_session(&mut self) -> Result<ChatSession> {
        let id = Uuid::new_v4().to_string();
        let now = unix_seconds();
        self.conn.execute(
            "INSERT INTO chat_sessions(id, title, created_at, updated_at) VALUES(?1, 'New chat', ?2, ?2)",
            params![id, now],
        )?;
        self.get_chat_session(&id)?.ok_or(DbError::NotFound)
    }

    pub fn get_chat_session(&self, id: &str) -> Result<Option<ChatSession>> {
        self.conn
            .query_row(
                "SELECT id, title, created_at, updated_at, archived_at FROM chat_sessions WHERE id = ?1",
                [id],
                read_session,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_chat_sessions(&self, archived: bool) -> Result<Vec<ChatSession>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, created_at, updated_at, archived_at FROM chat_sessions
             WHERE (archived_at IS NOT NULL) = ?1 ORDER BY updated_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([archived], read_session)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn rename_chat_session(&mut self, id: &str, title: &str) -> Result<ChatSession> {
        let title = title.trim();
        if title.is_empty() || title.len() > 120 {
            return Err(DbError::Invalid("chat title must be 1-120 bytes".into()));
        }
        if self.conn.execute(
            "UPDATE chat_sessions SET title = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, title, unix_seconds()],
        )? != 1
        {
            return Err(DbError::NotFound);
        }
        self.get_chat_session(id)?.ok_or(DbError::NotFound)
    }

    pub fn set_chat_archived(&mut self, id: &str, archived: bool) -> Result<ChatSession> {
        if self.conn.execute(
            "UPDATE chat_sessions SET archived_at = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, archived.then(unix_seconds), unix_seconds()],
        )? != 1
        {
            return Err(DbError::NotFound);
        }
        self.get_chat_session(id)?.ok_or(DbError::NotFound)
    }

    pub fn list_chat_messages(&self, session_id: &str) -> Result<Vec<ChatMessage>> {
        if self.get_chat_session(session_id)?.is_none() {
            return Err(DbError::NotFound);
        }
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, sequence, reply_to, role, content, status, provider, model,
                    error, created_at, updated_at FROM chat_messages
             WHERE session_id = ?1 ORDER BY sequence",
        )?;
        let rows = stmt.query_map([session_id], read_message)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn list_chat_events(&self, session_id: &str) -> Result<Vec<ChatEvent>> {
        if self.get_chat_session(session_id)?.is_none() {
            return Err(DbError::NotFound);
        }
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.assistant_message_id, e.ordinal, e.tool, e.arguments_json,
                    e.result_json, e.action_id, e.created_at FROM chat_events e
             JOIN chat_messages m ON m.id = e.assistant_message_id
             WHERE m.session_id = ?1 ORDER BY m.sequence, e.ordinal",
        )?;
        let rows = stmt.query_map([session_id], read_event)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn list_chat_actions(&self, session_id: &str) -> Result<Vec<AgentAction>> {
        if self.get_chat_session(session_id)?.is_none() {
            return Err(DbError::NotFound);
        }
        let mut stmt = self.conn.prepare(
            "SELECT a.id, a.tool, a.arguments_json, a.target_types_json, a.status,
                    a.created_at, a.error, a.chat_message_id, a.decision, a.decided_at FROM agent_actions a
             JOIN chat_messages m ON m.id = a.chat_message_id
             WHERE m.session_id = ?1 ORDER BY m.sequence, a.created_at, a.id",
        )?;
        let rows = stmt.query_map([session_id], read_agent_action)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn begin_chat_turn(
        &mut self,
        session_id: &str,
        question: &str,
        provider: &str,
        model: &str,
    ) -> Result<(ChatMessage, ChatMessage)> {
        let session = self
            .get_chat_session(session_id)?
            .ok_or(DbError::NotFound)?;
        if session.archived_at.is_some() {
            return Err(DbError::Invalid(
                "restore this chat before sending a message".into(),
            ));
        }
        let tx = self.conn.transaction()?;
        let next: i64 = tx.query_row(
            "SELECT COALESCE(MAX(sequence), 0) + 1 FROM chat_messages WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )?;
        let user_id = Uuid::new_v4().to_string();
        let assistant_id = Uuid::new_v4().to_string();
        let now = unix_seconds();
        tx.execute(
            "INSERT INTO chat_messages(id, session_id, sequence, role, content, status, created_at, updated_at)
             VALUES(?1, ?2, ?3, 'user', ?4, 'completed', ?5, ?5)",
            params![user_id, session_id, next, question, now],
        )?;
        tx.execute(
            "INSERT INTO chat_messages(id, session_id, sequence, reply_to, role, content, status,
                                       provider, model, created_at, updated_at)
             VALUES(?1, ?2, ?3, ?4, 'assistant', '', 'running', ?5, ?6, ?7, ?7)",
            params![
                assistant_id,
                session_id,
                next + 1,
                user_id,
                provider,
                model,
                now
            ],
        )?;
        if session.title == "New chat" {
            let title: String = question.chars().take(72).collect();
            tx.execute(
                "UPDATE chat_sessions SET title = ?2, updated_at = ?3 WHERE id = ?1",
                params![session_id, title, now],
            )?;
        } else {
            tx.execute(
                "UPDATE chat_sessions SET updated_at = ?2 WHERE id = ?1",
                params![session_id, now],
            )?;
        }
        tx.commit()?;
        let messages = self.list_chat_messages(session_id)?;
        Ok((
            messages[messages.len() - 2].clone(),
            messages[messages.len() - 1].clone(),
        ))
    }

    pub fn finish_chat_message(
        &mut self,
        id: &str,
        content: &str,
        error: Option<&str>,
    ) -> Result<()> {
        let status = if error.is_some() {
            "failed"
        } else {
            "completed"
        };
        if self.conn.execute(
            "UPDATE chat_messages SET content = ?2, status = ?3, error = ?4, updated_at = ?5
             WHERE id = ?1 AND role = 'assistant' AND status = 'running'",
            params![id, content, status, error, unix_seconds()],
        )? != 1
        {
            return Err(DbError::Conflict);
        }
        Ok(())
    }

    pub fn append_chat_event(
        &mut self,
        assistant_message_id: &str,
        ordinal: i64,
        tool: &str,
        arguments: &Value,
        result: &Value,
        action_id: Option<&str>,
    ) -> Result<ChatEvent> {
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO chat_events(id, assistant_message_id, ordinal, tool, arguments_json,
                                     result_json, action_id, created_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                assistant_message_id,
                ordinal,
                tool,
                arguments.to_string(),
                result.to_string(),
                action_id,
                unix_seconds()
            ],
        )?;
        self.conn
            .query_row(
                "SELECT id, assistant_message_id, ordinal, tool, arguments_json, result_json,
                    action_id, created_at FROM chat_events WHERE id = ?1",
                [&id],
                read_event,
            )
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn chats_and_action_receipts_survive_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.sqlite");
        let mut db = Db::open(&path).unwrap();
        let session = db.create_chat_session().unwrap();
        let (_, reply) = db
            .begin_chat_turn(&session.id, "Find labs", "ollama", "test")
            .unwrap();
        let action = db
            .queue_chat_agent_action(
                "create_record",
                &json!({"type_key":"note","title":"A"}),
                &["note".into()],
                Some(&reply.id),
            )
            .unwrap();
        db.append_chat_event(
            &reply.id,
            1,
            "create_record",
            &action.arguments,
            &json!({"status":"pending","action_id":action.id}),
            Some(&action.id),
        )
        .unwrap();
        db.finish_chat_message(&reply.id, "Waiting for approval", None)
            .unwrap();
        db.rename_chat_session(&session.id, "Lab search").unwrap();
        db.set_chat_archived(&session.id, true).unwrap();
        drop(db);

        let mut db = Db::open(&path).unwrap();
        assert!(db.list_chat_sessions(false).unwrap().is_empty());
        assert_eq!(db.list_chat_sessions(true).unwrap()[0].title, "Lab search");
        assert_eq!(db.list_chat_messages(&session.id).unwrap().len(), 2);
        assert_eq!(db.list_chat_events(&session.id).unwrap().len(), 1);
        assert_eq!(
            db.list_chat_actions(&session.id).unwrap()[0]
                .chat_message_id
                .as_deref(),
            Some(reply.id.as_str())
        );
        assert!(
            db.begin_chat_turn(&session.id, "Again", "ollama", "test")
                .is_err()
        );
        db.set_chat_archived(&session.id, false).unwrap();
        assert!(
            db.begin_chat_turn(&session.id, "Again", "ollama", "test")
                .is_ok()
        );
    }
}
