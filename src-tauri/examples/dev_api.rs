//! Loopback-only API for the browser development view. It opens the same SQLite
//! database as Tauri; production builds use native IPC and do not run this API.
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, StatusCode, header::ORIGIN};
use axum::routing::{get, post};
use axum::{Json, Router};
use eduport_lib::agent::{self, AskInput};
use eduport_lib::ai::{self, AiConfig};
use eduport_lib::db::{
    Db, RecordInput, RecordType, RelationDefinitionInput, RelationInput, TableViewInput,
};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

#[derive(Deserialize)]
struct CommandRequest {
    command: String,
    #[serde(default)]
    args: Value,
}

type SharedDb = Arc<Mutex<Db>>;

#[tokio::main]
async fn main() {
    let path =
        PathBuf::from(std::env::var("EDUPORT_DB_PATH").expect("EDUPORT_DB_PATH must be set"));
    let db = Db::open(&path).expect("open Eduport SQLite database");
    let app = Router::new()
        .route("/health", get(|| async { Json(json!({ "ok": true })) }))
        .route("/api/command", post(command))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .with_state(Arc::new(Mutex::new(db)));
    let api_port = std::env::var("EDUPORT_DEV_API_PORT").unwrap_or_else(|_| "1422".into());
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{api_port}"))
        .await
        .expect("bind local development API");
    eprintln!("Eduport local SQLite API ready at 127.0.0.1:{api_port}");
    axum::serve(listener, app)
        .await
        .expect("serve local development API");
}

async fn command(
    State(state): State<SharedDb>,
    headers: HeaderMap,
    Json(request): Json<CommandRequest>,
) -> (StatusCode, Json<Value>) {
    let expected_origin =
        std::env::var("EDUPORT_UI_ORIGIN").unwrap_or_else(|_| "http://127.0.0.1:1420".into());
    if headers
        .get(ORIGIN)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|origin| origin != expected_origin)
    {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Unexpected browser origin." })),
        );
    }
    let result = if request.command == "ask_assistant" {
        match argument::<AskInput>(&request.args, "input") {
            Ok(input) => agent::ask(&state, input).await.and_then(as_value),
            Err(error) => Err(error),
        }
    } else {
        match state.lock() {
            Ok(mut db) => dispatch(&mut db, &request.command, &request.args),
            Err(_) => Err("Database lock was poisoned.".into()),
        }
    };
    match result {
        Ok(value) => (StatusCode::OK, Json(json!({ "result": value }))),
        Err(error) => (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))),
    }
}

fn argument<T: DeserializeOwned>(args: &Value, key: &str) -> Result<T, String> {
    serde_json::from_value(args.get(key).cloned().unwrap_or(Value::Null))
        .map_err(|error| format!("Invalid {key}: {error}"))
}

fn as_value<T: serde::Serialize>(value: T) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|error| error.to_string())
}

fn dispatch(db: &mut Db, command: &str, args: &Value) -> Result<Value, String> {
    match command {
        "list_types" => as_value(db.list_types().map_err(|e| e.to_string())?),
        "put_type" => as_value(
            db.put_type(argument::<RecordType>(args, "definition")?)
                .map_err(|e| e.to_string())?,
        ),
        "list_records" => {
            let type_key = args.get("typeKey").and_then(Value::as_str);
            let include_deleted = args
                .get("includeDeleted")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            as_value(
                db.list_records(type_key, include_deleted)
                    .map_err(|e| e.to_string())?,
            )
        }
        "get_record" => as_value(
            db.get_record(&argument::<String>(args, "id")?)
                .map_err(|e| e.to_string())?,
        ),
        "put_record" => as_value(
            db.put_record(argument::<RecordInput>(args, "input")?)
                .map_err(|e| e.to_string())?,
        ),
        "set_deleted" => as_value(
            db.set_deleted(
                &argument::<String>(args, "id")?,
                argument::<bool>(args, "deleted")?,
            )
            .map_err(|e| e.to_string())?,
        ),
        "search_records" => as_value(
            db.search(
                &argument::<String>(args, "query")?,
                args.get("limit").and_then(Value::as_u64).unwrap_or(30) as usize,
            )
            .map_err(|e| e.to_string())?,
        ),
        "list_relations" => as_value(
            db.list_relations(&argument::<String>(args, "id")?)
                .map_err(|e| e.to_string())?,
        ),
        "list_all_relations" => as_value(db.all_relations().map_err(|e| e.to_string())?),
        "put_relation" => as_value(
            db.put_relation(argument::<RelationInput>(args, "input")?)
                .map_err(|e| e.to_string())?,
        ),
        "delete_relation" => as_value(
            db.delete_relation(&argument::<String>(args, "id")?)
                .map_err(|e| e.to_string())?,
        ),
        "list_relation_definitions" => as_value(
            db.list_relation_definitions(args.get("typeKey").and_then(Value::as_str))
                .map_err(|e| e.to_string())?,
        ),
        "put_relation_definition" => as_value(
            db.put_relation_definition(argument::<RelationDefinitionInput>(args, "input")?)
                .map_err(|e| e.to_string())?,
        ),
        "list_views" => as_value(
            db.list_views(&argument::<String>(args, "typeKey")?)
                .map_err(|e| e.to_string())?,
        ),
        "put_view" => as_value(
            db.put_view(argument::<TableViewInput>(args, "input")?)
                .map_err(|e| e.to_string())?,
        ),
        "delete_view" => as_value(
            db.delete_view(&argument::<String>(args, "id")?)
                .map_err(|e| e.to_string())?,
        ),
        "get_setting" => as_value(
            db.get_setting(&argument::<String>(args, "key")?)
                .map_err(|e| e.to_string())?,
        ),
        "put_setting" => {
            db.put_setting(
                &argument::<String>(args, "key")?,
                &argument::<Value>(args, "value")?,
            )
            .map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        "get_ai_config" => as_value(ai::load_config(db)?),
        "put_ai_config" => {
            ai::save_config(db, &argument::<AiConfig>(args, "config")?)?;
            Ok(Value::Null)
        }
        "set_openrouter_key" => {
            ai::save_openrouter_key(&argument::<String>(args, "key")?)?;
            Ok(Value::Null)
        }
        "has_openrouter_key" => as_value(ai::has_openrouter_key()?),
        "create_chat_session" => as_value(db.create_chat_session().map_err(|e| e.to_string())?),
        "list_chat_sessions" => as_value(
            db.list_chat_sessions(
                args.get("archived")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            )
            .map_err(|e| e.to_string())?,
        ),
        "rename_chat_session" => as_value(
            db.rename_chat_session(
                &argument::<String>(args, "id")?,
                &argument::<String>(args, "title")?,
            )
            .map_err(|e| e.to_string())?,
        ),
        "set_chat_archived" => as_value(
            db.set_chat_archived(
                &argument::<String>(args, "id")?,
                argument::<bool>(args, "archived")?,
            )
            .map_err(|e| e.to_string())?,
        ),
        "list_chat_messages" => as_value(
            db.list_chat_messages(&argument::<String>(args, "sessionId")?)
                .map_err(|e| e.to_string())?,
        ),
        "list_chat_events" => as_value(
            db.list_chat_events(&argument::<String>(args, "sessionId")?)
                .map_err(|e| e.to_string())?,
        ),
        "list_chat_actions" => as_value(
            db.list_chat_actions(&argument::<String>(args, "sessionId")?)
                .map_err(|e| e.to_string())?,
        ),
        "list_agent_actions" => as_value(db.list_agent_actions().map_err(|e| e.to_string())?),
        "approve_agent_action" => as_value(agent::approve(db, &argument::<String>(args, "id")?)?),
        "reject_agent_action" => as_value(agent::reject(db, &argument::<String>(args, "id")?)?),
        "get_gmail_account" => {
            as_value(db.get_setting("gmail_account").map_err(|e| e.to_string())?)
        }
        _ => Err("This action is available in the desktop app.".into()),
    }
}
