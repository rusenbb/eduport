#![forbid(unsafe_code)]

pub mod agent;
pub mod ai;
pub mod db;
mod mail;
mod portable;

use std::sync::Mutex;

use db::{
    Db, Record, RecordInput, RecordType, Relation, RelationDefinition, RelationDefinitionInput,
    RelationInput, TableView, TableViewInput,
};
use serde_json::Value;
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

type DbState = Mutex<Db>;
type PendingImport = Mutex<Option<portable::ImportPlan>>;
type PendingMail = Mutex<Option<mail::PendingMail>>;

fn with_db<T>(
    state: tauri::State<'_, DbState>,
    f: impl FnOnce(&mut Db) -> db::Result<T>,
) -> Result<T, String> {
    let mut guard = state
        .lock()
        .map_err(|_| "database lock was poisoned".to_string())?;
    f(&mut guard).map_err(|error| error.to_string())
}

#[tauri::command]
fn list_types(state: tauri::State<'_, DbState>) -> Result<Vec<RecordType>, String> {
    with_db(state, |db| db.list_types())
}

#[tauri::command]
fn put_type(
    state: tauri::State<'_, DbState>,
    definition: RecordType,
) -> Result<RecordType, String> {
    with_db(state, |db| db.put_type(definition))
}

#[tauri::command]
fn list_records(
    state: tauri::State<'_, DbState>,
    type_key: Option<String>,
    include_deleted: Option<bool>,
) -> Result<Vec<Record>, String> {
    with_db(state, |db| {
        db.list_records(type_key.as_deref(), include_deleted.unwrap_or(false))
    })
}

#[tauri::command]
fn get_record(state: tauri::State<'_, DbState>, id: String) -> Result<Option<Record>, String> {
    with_db(state, |db| db.get_record(&id))
}

#[tauri::command]
fn put_record(state: tauri::State<'_, DbState>, input: RecordInput) -> Result<Record, String> {
    with_db(state, |db| db.put_record(input))
}

#[tauri::command]
fn set_deleted(
    state: tauri::State<'_, DbState>,
    id: String,
    deleted: bool,
) -> Result<Record, String> {
    with_db(state, |db| db.set_deleted(&id, deleted))
}

#[tauri::command]
fn search_records(
    state: tauri::State<'_, DbState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<Record>, String> {
    with_db(state, |db| db.search(&query, limit.unwrap_or(30)))
}

#[tauri::command]
fn list_relations(state: tauri::State<'_, DbState>, id: String) -> Result<Vec<Relation>, String> {
    with_db(state, |db| db.list_relations(&id))
}

#[tauri::command]
fn list_all_relations(state: tauri::State<'_, DbState>) -> Result<Vec<Relation>, String> {
    with_db(state, |db| db.all_relations())
}

#[tauri::command]
fn put_relation(
    state: tauri::State<'_, DbState>,
    input: RelationInput,
) -> Result<Relation, String> {
    with_db(state, |db| db.put_relation(input))
}

#[tauri::command]
fn delete_relation(state: tauri::State<'_, DbState>, id: String) -> Result<(), String> {
    with_db(state, |db| db.delete_relation(&id))
}

#[tauri::command]
fn list_relation_definitions(
    state: tauri::State<'_, DbState>,
    type_key: Option<String>,
) -> Result<Vec<RelationDefinition>, String> {
    with_db(state, |db| {
        db.list_relation_definitions(type_key.as_deref())
    })
}

#[tauri::command]
fn put_relation_definition(
    state: tauri::State<'_, DbState>,
    input: RelationDefinitionInput,
) -> Result<RelationDefinition, String> {
    with_db(state, |db| db.put_relation_definition(input))
}

#[tauri::command]
fn list_views(
    state: tauri::State<'_, DbState>,
    type_key: String,
) -> Result<Vec<TableView>, String> {
    with_db(state, |db| db.list_views(&type_key))
}

#[tauri::command]
fn put_view(state: tauri::State<'_, DbState>, input: TableViewInput) -> Result<TableView, String> {
    with_db(state, |db| db.put_view(input))
}

#[tauri::command]
fn delete_view(state: tauri::State<'_, DbState>, id: String) -> Result<(), String> {
    with_db(state, |db| db.delete_view(&id))
}

#[tauri::command]
fn get_setting(state: tauri::State<'_, DbState>, key: String) -> Result<Option<Value>, String> {
    with_db(state, |db| db.get_setting(&key))
}

#[tauri::command]
fn put_setting(state: tauri::State<'_, DbState>, key: String, value: Value) -> Result<(), String> {
    with_db(state, |db| db.put_setting(&key, &value))
}

#[tauri::command]
fn preview_markdown_import(
    app: tauri::AppHandle,
    state: tauri::State<'_, DbState>,
    pending: tauri::State<'_, PendingImport>,
) -> Result<Option<portable::ImportPreview>, String> {
    let Some(folder) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    let path = folder.into_path().map_err(|e| e.to_string())?;
    let types = with_db(state, |db| db.list_types())?;
    let plan = portable::scan(&path, &types)?;
    let preview = plan.preview.clone();
    *pending
        .lock()
        .map_err(|_| "import lock poisoned".to_string())? = Some(plan);
    Ok(Some(preview))
}

#[tauri::command]
fn apply_markdown_import(
    state: tauri::State<'_, DbState>,
    pending: tauri::State<'_, PendingImport>,
    fingerprint: String,
) -> Result<usize, String> {
    let plan = {
        let mut guard = pending
            .lock()
            .map_err(|_| "import lock poisoned".to_string())?;
        if guard.as_ref().map(|plan| &plan.preview.fingerprint) != Some(&fingerprint) {
            return Err("Preview this folder again before importing.".into());
        }
        guard
            .take()
            .ok_or_else(|| "No pending import.".to_string())?
    };
    let mut db = state
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    portable::apply(&mut db, plan)
}

#[tauri::command]
fn export_markdown(
    app: tauri::AppHandle,
    state: tauri::State<'_, DbState>,
) -> Result<Option<String>, String> {
    let Some(folder) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    let parent = folder.into_path().map_err(|e| e.to_string())?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let destination = parent.join(format!("eduport-export-{stamp}"));
    let db = state
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    portable::export_markdown(&db, &destination)?;
    Ok(Some(destination.to_string_lossy().into_owned()))
}

#[tauri::command]
fn backup_database(
    app: tauri::AppHandle,
    state: tauri::State<'_, DbState>,
) -> Result<Option<String>, String> {
    let Some(file) = app
        .dialog()
        .file()
        .add_filter("SQLite database", &["sqlite"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = file.into_path().map_err(|e| e.to_string())?;
    with_db(state, |db| db.backup_to(&path))?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

#[tauri::command]
fn get_ai_config(state: tauri::State<'_, DbState>) -> Result<ai::AiConfig, String> {
    let db = state
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    ai::load_config(&db)
}

#[tauri::command]
fn put_ai_config(state: tauri::State<'_, DbState>, config: ai::AiConfig) -> Result<(), String> {
    let mut db = state
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    ai::save_config(&mut db, &config)
}

#[tauri::command]
fn set_openrouter_key(key: String) -> Result<(), String> {
    ai::save_openrouter_key(&key)
}

#[tauri::command]
fn has_openrouter_key() -> Result<bool, String> {
    ai::has_openrouter_key()
}

#[tauri::command]
async fn ask_assistant(
    state: tauri::State<'_, DbState>,
    input: agent::AskInput,
) -> Result<agent::AskResult, String> {
    agent::ask(&state, input).await
}

#[tauri::command]
fn list_agent_actions(state: tauri::State<'_, DbState>) -> Result<Vec<db::AgentAction>, String> {
    with_db(state, |db| db.list_agent_actions())
}

#[tauri::command]
fn approve_agent_action(
    app: tauri::AppHandle,
    state: tauri::State<'_, DbState>,
    id: String,
) -> Result<agent::ActionOutcome, String> {
    let action = {
        let db = state
            .lock()
            .map_err(|_| "database lock poisoned".to_string())?;
        db.get_agent_action(&id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Action not found.".to_string())?
    };
    let approved = app
        .dialog()
        .message(format!(
            "Apply AI action '{}' to {}? Review its arguments in Eduport first.",
            action.tool,
            action.target_types.join(", ")
        ))
        .title("Approve AI database change")
        .buttons(MessageDialogButtons::OkCancel)
        .blocking_show();
    if !approved {
        return Err("Action was not approved.".into());
    }
    let mut db = state
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    agent::approve(&mut db, &id)
}

#[tauri::command]
fn reject_agent_action(
    state: tauri::State<'_, DbState>,
    id: String,
) -> Result<db::AgentAction, String> {
    let mut db = state
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    agent::reject(&mut db, &id)
}

#[tauri::command]
fn get_gmail_account(
    state: tauri::State<'_, DbState>,
) -> Result<Option<mail::GmailAccount>, String> {
    let db = state
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    load_gmail_account(&db)
}

fn load_gmail_account(db: &Db) -> Result<Option<mail::GmailAccount>, String> {
    db.get_setting("gmail_account")
        .map_err(|e| e.to_string())?
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn connect_gmail(
    app: tauri::AppHandle,
    state: tauri::State<'_, DbState>,
    client_id: String,
) -> Result<mail::GmailAccount, String> {
    let account = mail::connect(&app, &client_id).await?;
    let mut db = state
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    db.put_setting(
        "gmail_account",
        &serde_json::to_value(&account).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(account)
}

#[tauri::command]
async fn search_gmail(
    state: tauri::State<'_, DbState>,
    query: String,
) -> Result<Vec<mail::GmailMessage>, String> {
    let (account, imported) = {
        let db = state
            .lock()
            .map_err(|_| "database lock poisoned".to_string())?;
        let account = load_gmail_account(&db)?.ok_or("Connect Gmail first.")?;
        let imported = db
            .imported_message_ids("gmail", &account.email)
            .map_err(|e| e.to_string())?;
        (account, imported)
    };
    mail::search(&account, &query, &imported).await
}

#[tauri::command]
async fn preview_gmail_import(
    state: tauri::State<'_, DbState>,
    pending: tauri::State<'_, PendingMail>,
    ids: Vec<String>,
) -> Result<Vec<mail::MailPreview>, String> {
    let (account, imported) = {
        let db = state
            .lock()
            .map_err(|_| "database lock poisoned".to_string())?;
        let account = load_gmail_account(&db)?.ok_or("Connect Gmail first.")?;
        let imported = db
            .imported_message_ids("gmail", &account.email)
            .map_err(|e| e.to_string())?;
        (account, imported)
    };
    let (preview, plan) = mail::preview(&account, &ids, &imported).await?;
    *pending
        .lock()
        .map_err(|_| "mail import lock poisoned".to_string())? = Some(plan);
    Ok(preview)
}

#[tauri::command]
fn import_gmail_selected(
    state: tauri::State<'_, DbState>,
    pending: tauri::State<'_, PendingMail>,
    ids: Vec<String>,
) -> Result<usize, String> {
    let plan = mail::take_pending(&pending, &ids)?;
    let mut db = state
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let account = load_gmail_account(&db)?.ok_or("Connect Gmail first.")?;
    if account.email != plan.account {
        return Err("Gmail account changed. Preview messages again.".into());
    }
    db.import_external_mail(&plan.account, &plan.messages)
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let path = app.path().app_data_dir()?.join("eduport.sqlite");
            let db = Db::open(&path)
                .map_err(|error| format!("database at {}: {error}", path.display()))?;
            app.manage::<DbState>(Mutex::new(db));
            app.manage::<PendingImport>(Mutex::new(None));
            app.manage::<PendingMail>(Mutex::new(None));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_types,
            put_type,
            list_records,
            get_record,
            put_record,
            set_deleted,
            search_records,
            list_relations,
            list_all_relations,
            put_relation,
            delete_relation,
            list_relation_definitions,
            put_relation_definition,
            list_views,
            put_view,
            delete_view,
            get_setting,
            put_setting,
            preview_markdown_import,
            apply_markdown_import,
            export_markdown,
            backup_database,
            get_ai_config,
            put_ai_config,
            set_openrouter_key,
            has_openrouter_key,
            ask_assistant,
            list_agent_actions,
            approve_agent_action,
            reject_agent_action,
            get_gmail_account,
            connect_gmail,
            search_gmail,
            preview_gmail_import,
            import_gmail_selected,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start Eduport");
}
