use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::db::{Db, FieldDef, Record, RecordType, Relation, RelationDefinition, TableView};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct PreviewRow {
    pub id: String,
    pub type_key: String,
    pub title: String,
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportPreview {
    pub fingerprint: String,
    pub format: String,
    pub rows: Vec<PreviewRow>,
    pub file_count: usize,
    pub warnings: Vec<String>,
    pub blockers: Vec<String>,
}

pub struct ImportPlan {
    pub source: PathBuf,
    pub preview: ImportPreview,
    types: Vec<RecordType>,
    records: Vec<Record>,
    relation_definitions: Vec<RelationDefinition>,
    relations: Vec<Relation>,
    views: Vec<TableView>,
    files: Vec<(String, Vec<u8>)>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    version: u32,
    types: Vec<RecordType>,
    #[serde(default)]
    relation_definitions: Vec<RelationDefinition>,
    relations: Vec<Relation>,
    #[serde(default)]
    views: Vec<TableView>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Frontmatter {
    id: String,
    type_key: String,
    title: String,
    #[serde(default)]
    values: Value,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default = "one")]
    revision: i64,
    #[serde(default)]
    created_at: i64,
    #[serde(default)]
    updated_at: i64,
    #[serde(default)]
    deleted_at: Option<i64>,
}

fn one() -> i64 {
    1
}

pub fn export_markdown(db: &Db, destination: &Path) -> Result<usize, String> {
    if destination.exists() {
        return Err("Export destination already exists.".into());
    }
    let parent = destination
        .parent()
        .ok_or_else(|| "Export destination needs a parent folder.".to_string())?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let stage = tempfile::Builder::new()
        .prefix(".eduport-export-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;

    let records = db.list_records(None, true).map_err(|e| e.to_string())?;
    let manifest = Manifest {
        version: 2,
        types: db.list_types().map_err(|e| e.to_string())?,
        relation_definitions: db
            .list_relation_definitions(None)
            .map_err(|e| e.to_string())?,
        relations: db.all_relations().map_err(|e| e.to_string())?,
        views: db.all_custom_views().map_err(|e| e.to_string())?,
    };
    fs::write(
        stage.path().join("eduport-export.yaml"),
        serde_yaml::to_string(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    for record in &records {
        if !safe_segment(&record.id) || !safe_segment(&record.type_key) {
            return Err(format!("Unsafe record path for {}", record.id));
        }
        let frontmatter = Frontmatter {
            id: record.id.clone(),
            type_key: record.type_key.clone(),
            title: record.title.clone(),
            values: record.values.clone(),
            tags: record.tags.clone(),
            revision: record.revision,
            created_at: record.created_at,
            updated_at: record.updated_at,
            deleted_at: record.deleted_at,
        };
        let folder = stage.path().join("records").join(&record.type_key);
        fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let yaml = serde_yaml::to_string(&frontmatter).map_err(|e| e.to_string())?;
        fs::write(
            folder.join(format!("{}.md", record.id)),
            format!("---\n{}---\n{}", yaml, record.body),
        )
        .map_err(|e| e.to_string())?;
    }
    for (relative_path, bytes) in db.all_files().map_err(|e| e.to_string())? {
        let relative = safe_relative_path(&relative_path)?;
        let target = stage.path().join("files").join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(target, bytes).map_err(|e| e.to_string())?;
    }
    fs::rename(stage.path(), destination).map_err(|e| e.to_string())?;
    Ok(records.len())
}

pub fn scan(source: &Path, existing_types: &[RecordType]) -> Result<ImportPlan, String> {
    if !source.is_dir() {
        return Err("Choose a folder containing an Eduport export or Markdown records.".into());
    }
    let manifest_path = source.join("eduport-export.yaml");
    if manifest_path.exists() {
        scan_export(source)
    } else {
        scan_legacy(source, existing_types)
    }
}

fn scan_export(source: &Path) -> Result<ImportPlan, String> {
    let source_files = collect_files(source)?;
    let fingerprint = fingerprint(source, &source_files)?;
    let manifest: Manifest = serde_yaml::from_slice(
        &fs::read(source.join("eduport-export.yaml")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if !matches!(manifest.version, 1 | 2) {
        return Err(format!(
            "Unsupported Eduport export version {}.",
            manifest.version
        ));
    }
    let mut records = Vec::new();
    let mut files = Vec::new();
    let mut blockers = Vec::new();
    for path in source_files {
        let relative = path.strip_prefix(source).map_err(|e| e.to_string())?;
        let parts: Vec<_> = relative.components().collect();
        if parts
            .first()
            .is_some_and(|part| part.as_os_str() == "records")
            && path.extension().and_then(|extension| extension.to_str()) == Some("md")
        {
            match read_record_file(&path) {
                Ok(record) => records.push(record),
                Err(error) => blockers.push(format!("{}: {error}", relative.display())),
            }
        } else if parts
            .first()
            .is_some_and(|part| part.as_os_str() == "files")
        {
            let stored_path = relative.strip_prefix("files").map_err(|e| e.to_string())?;
            let name = stored_path.to_string_lossy().replace('\\', "/");
            if safe_relative_path(&name).is_err() {
                blockers.push(format!("Unsafe file path: {}", relative.display()));
                continue;
            }
            files.push((name, fs::read(&path).map_err(|e| e.to_string())?));
        }
    }
    let preview = ImportPreview {
        fingerprint,
        format: "Eduport Markdown export".into(),
        rows: rows(&records),
        file_count: files.len(),
        warnings: Vec::new(),
        blockers,
    };
    Ok(ImportPlan {
        source: source.to_path_buf(),
        preview,
        types: manifest.types,
        relation_definitions: manifest.relation_definitions,
        records,
        relations: manifest.relations,
        views: manifest.views,
        files,
    })
}

fn read_record_file(path: &Path) -> Result<Record, String> {
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let (yaml, body) = split_frontmatter(&raw)?;
    let frontmatter: Frontmatter = serde_yaml::from_str(yaml).map_err(|e| e.to_string())?;
    if !safe_segment(&frontmatter.id) || !safe_segment(&frontmatter.type_key) {
        return Err("Unsafe ID or type key.".into());
    }
    Ok(Record {
        id: frontmatter.id,
        type_key: frontmatter.type_key,
        title: frontmatter.title,
        body: body.to_string(),
        values: frontmatter.values,
        tags: frontmatter.tags,
        revision: frontmatter.revision,
        created_at: frontmatter.created_at,
        updated_at: frontmatter.updated_at,
        deleted_at: frontmatter.deleted_at,
    })
}

fn split_frontmatter(raw: &str) -> Result<(&str, &str), String> {
    let rest = raw
        .strip_prefix("---\n")
        .ok_or_else(|| "Missing opening frontmatter marker.".to_string())?;
    let end = rest
        .find("\n---\n")
        .ok_or_else(|| "Missing closing frontmatter marker.".to_string())?;
    Ok((&rest[..end], &rest[end + 5..]))
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    fn visit(path: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_symlink() {
                return Err(format!(
                    "Symlink is not imported: {}",
                    entry.path().display()
                ));
            }
            if kind.is_dir() {
                visit(&entry.path(), output)?;
            } else if kind.is_file() {
                output.push(entry.path());
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    visit(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn fingerprint(root: &Path, files: &[PathBuf]) -> Result<String, String> {
    let mut hasher = Sha256::new();
    for path in files {
        let relative = path.strip_prefix(root).map_err(|e| e.to_string())?;
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        hasher.update(relative.as_os_str().as_encoded_bytes());
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn rows(records: &[Record]) -> Vec<PreviewRow> {
    records
        .iter()
        .map(|record| PreviewRow {
            id: record.id.clone(),
            type_key: record.type_key.clone(),
            title: record.title.clone(),
            deleted: record.deleted_at.is_some(),
        })
        .collect()
}

fn safe_segment(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn safe_relative_path(text: &str) -> Result<&Path, String> {
    let path = Path::new(text);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || !path
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
    {
        return Err(format!("Unsafe relative path: {text}"));
    }
    Ok(path)
}

pub fn apply(db: &mut Db, plan: ImportPlan) -> Result<usize, String> {
    if !plan.preview.blockers.is_empty() {
        return Err("Resolve import blockers before applying.".into());
    }
    let fresh = scan(&plan.source, &db.list_types().map_err(|e| e.to_string())?)?;
    if fresh.preview.fingerprint != plan.preview.fingerprint {
        return Err("Source changed after preview. Preview it again.".into());
    }
    db.import_snapshot(
        &fresh.types,
        &fresh.records,
        &fresh.relation_definitions,
        &fresh.relations,
        &fresh.views,
        &fresh.files,
    )
    .map_err(|e| e.to_string())?;
    Ok(fresh.records.len())
}

fn scan_legacy(source: &Path, existing_types: &[RecordType]) -> Result<ImportPlan, String> {
    let source_files = collect_files(source)?;
    let fingerprint = fingerprint(source, &source_files)?;
    let mut types = existing_types.to_vec();
    let mut records = Vec::new();
    let mut files = Vec::new();
    let mut warnings = Vec::new();
    let mut blockers = Vec::new();
    let mut pending_links: Vec<(String, String, String, Option<i64>)> = Vec::new();

    for path in source_files {
        let relative = path.strip_prefix(source).map_err(|e| e.to_string())?;
        let components: Vec<_> = relative.components().collect();
        let first = components
            .first()
            .map(|part| part.as_os_str().to_string_lossy().to_string());
        let extension = path.extension().and_then(|value| value.to_str());
        let is_root_record = components.len() == 1 && extension == Some("md");
        let is_trash_record =
            first.as_deref() == Some(".trash") && components.len() == 2 && extension == Some("md");
        let is_note = first.as_deref() == Some("notes") && extension == Some("md");
        if is_root_record || is_trash_record || is_note {
            let bytes = fs::read(&path).map_err(|e| e.to_string())?;
            let raw = match String::from_utf8(bytes.clone()) {
                Ok(raw) => raw,
                Err(error) => {
                    blockers.push(format!("{} is not UTF-8: {error}", relative.display()));
                    continue;
                }
            };
            let id = path
                .file_stem()
                .and_then(|part| part.to_str())
                .unwrap_or_default();
            if !safe_segment(id) {
                blockers.push(format!("Unsafe Markdown filename: {}", relative.display()));
                continue;
            }
            let (yaml, body) = match split_frontmatter(&raw) {
                Ok(parts) => (Some(parts.0), parts.1.to_string()),
                Err(_) if is_note => (None, raw.clone()),
                Err(error) => {
                    blockers.push(format!("{}: {error}", relative.display()));
                    continue;
                }
            };
            let metadata: serde_yaml::Value = match yaml {
                Some(text) => match serde_yaml::from_str(text) {
                    Ok(value) => value,
                    Err(error) => {
                        blockers.push(format!("{}: {error}", relative.display()));
                        continue;
                    }
                },
                None => serde_yaml::Value::Mapping(Default::default()),
            };
            let Some(mapping) = metadata.as_mapping() else {
                blockers.push(format!("{}: frontmatter must be a map", relative.display()));
                continue;
            };
            let tags: Vec<String> = mapping
                .get(serde_yaml::Value::String("tags".into()))
                .and_then(serde_yaml::Value::as_sequence)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            let declared_type = tags
                .iter()
                .find_map(|tag| tag.strip_prefix("eduport-type/"))
                .unwrap_or(if is_note { "note" } else { "" });
            if declared_type.is_empty() {
                blockers.push(format!("{}: no Eduport type tag", relative.display()));
                continue;
            }
            let type_key = if declared_type == "email" {
                "mail".to_string()
            } else {
                declared_type.to_string()
            };
            if !safe_segment(&type_key) {
                blockers.push(format!("{}: invalid type tag", relative.display()));
                continue;
            }
            if !types.iter().any(|kind| kind.key == type_key) {
                types.push(RecordType {
                    key: type_key.clone(),
                    label: type_key.clone(),
                    icon: "file".into(),
                    fields: Vec::new(),
                    built_in: false,
                });
            }
            let title = mapping
                .get(serde_yaml::Value::String("name".into()))
                .or_else(|| mapping.get(serde_yaml::Value::String("title".into())))
                .and_then(serde_yaml::Value::as_str)
                .unwrap_or(id)
                .to_string();
            let mut values = serde_json::Map::new();
            for (raw_key, raw_value) in mapping {
                let Some(key) = raw_key.as_str() else {
                    warnings.push(format!("{} has a non-text field key", relative.display()));
                    continue;
                };
                if matches!(key, "name" | "title" | "tags") {
                    continue;
                }
                let normalized = normalize_field_key(key);
                if normalized != key {
                    warnings.push(format!(
                        "{}: field {key:?} imported as {normalized:?}",
                        relative.display()
                    ));
                }
                let converted = yaml_value(raw_value);
                let (value, field_kind) = match converted {
                    Value::Null => continue,
                    Value::Bool(value) => (Value::Bool(value), "boolean"),
                    Value::Number(value) => (Value::Number(value), "number"),
                    Value::String(value) => (Value::String(value), "text"),
                    other => {
                        warnings.push(format!(
                            "{}: complex field {key:?} retained as text",
                            relative.display()
                        ));
                        (Value::String(other.to_string()), "text")
                    }
                };
                let definition = types.iter_mut().find(|kind| kind.key == type_key).unwrap();
                if let Some(field) = definition
                    .fields
                    .iter_mut()
                    .find(|field| field.key == normalized)
                {
                    if field.kind == "select"
                        && let Some(selected) = value.as_str()
                        && !field.options.iter().any(|option| option == selected)
                    {
                        field.options.push(selected.to_owned());
                    }
                    if field.kind != field_kind && field.kind != "select" {
                        warnings.push(format!(
                            "{}: {key:?} converted to text to fit existing field",
                            relative.display()
                        ));
                    }
                } else {
                    definition.fields.push(FieldDef {
                        key: normalized.clone(),
                        label: key.replace('_', " "),
                        kind: field_kind.into(),
                        options: Vec::new(),
                        required: false,
                    });
                }
                let stored = if let Some(field) = definition
                    .fields
                    .iter()
                    .find(|field| field.key == normalized)
                    && field.kind == "text"
                    && !value.is_string()
                {
                    Value::String(value.to_string())
                } else {
                    value
                };
                values.insert(normalized.clone(), stored);
                collect_wikilinks(raw_value, &mut pending_links, id, &normalized);
            }
            let user_tags = tags
                .into_iter()
                .filter(|tag| !tag.starts_with("eduport-type/"))
                .collect();
            records.push(Record {
                id: id.into(),
                type_key,
                title,
                body,
                values: Value::Object(values),
                tags: user_tags,
                revision: 1,
                created_at: 0,
                updated_at: 0,
                deleted_at: if is_trash_record { Some(0) } else { None },
            });
            files.push((
                "legacy-original/".to_owned() + &relative.to_string_lossy().replace('\\', "/"),
                bytes,
            ));
        } else if first.as_deref() == Some("attachments") {
            files.push((
                relative.to_string_lossy().replace('\\', "/"),
                fs::read(&path).map_err(|e| e.to_string())?,
            ));
        } else if relative == Path::new(".eduport/schema.yaml")
            || relative == Path::new(".eduport/views.yaml")
        {
            files.push((
                "legacy-original/".to_owned() + &relative.to_string_lossy().replace('\\', "/"),
                fs::read(&path).map_err(|e| e.to_string())?,
            ));
            warnings.push(format!(
                "{} preserved in the recovery export; custom fields with no values need review",
                relative.display()
            ));
        }
    }
    let mut seen = std::collections::HashSet::new();
    for record in &records {
        if !seen.insert(&record.id) {
            blockers.push(format!("Duplicate record ID: {}", record.id));
        }
    }
    let mut relations = Vec::new();
    for (source_id, kind, target, position) in pending_links {
        let matches: Vec<_> = records
            .iter()
            .filter(|record| record.id == target || record.title == target)
            .collect();
        if matches.len() == 1 {
            relations.push(Relation {
                id: Uuid::new_v4().to_string(),
                source_id,
                target_id: matches[0].id.clone(),
                kind,
                role: None,
                position,
                start_date: None,
                end_date: None,
                definition_id: None,
            });
        } else {
            warnings.push(format!(
                "Unresolved or ambiguous link: {} → {}",
                source_id, target
            ));
        }
    }
    let preview = ImportPreview {
        fingerprint,
        format: "Legacy Eduport Markdown vault".into(),
        rows: rows(&records),
        file_count: files.len(),
        warnings,
        blockers,
    };
    Ok(ImportPlan {
        source: source.to_path_buf(),
        preview,
        types,
        relation_definitions: Vec::new(),
        records,
        relations,
        views: Vec::new(),
        files,
    })
}

fn normalize_field_key(key: &str) -> String {
    let normalized: String = key
        .chars()
        .map(|letter| {
            if letter.is_ascii_alphanumeric() || letter == '_' {
                letter.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if normalized.is_empty() {
        "legacy_field".into()
    } else {
        normalized
    }
}

fn yaml_value(value: &serde_yaml::Value) -> Value {
    serde_json::to_value(value)
        .unwrap_or_else(|_| Value::String(serde_yaml::to_string(value).unwrap_or_default()))
}

fn collect_wikilinks(
    value: &serde_yaml::Value,
    output: &mut Vec<(String, String, String, Option<i64>)>,
    source_id: &str,
    kind: &str,
) {
    let strings: Vec<&str> = match value {
        serde_yaml::Value::String(text) => vec![text],
        serde_yaml::Value::Sequence(items) => {
            items.iter().filter_map(|item| item.as_str()).collect()
        }
        _ => Vec::new(),
    };
    for (position, text) in strings.into_iter().enumerate() {
        if let Some(target) = text
            .strip_prefix("[[")
            .and_then(|rest| rest.strip_suffix("]]"))
        {
            output.push((
                source_id.into(),
                kind.into(),
                target.into(),
                Some(position as i64),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{
        FilterRule, RecordInput, RelationDefinitionInput, RelationInput, SortRule, TableViewInput,
        ViewConfig,
    };

    #[test]
    fn export_preview_import_round_trip_and_stale_source_rejection() {
        let root = tempfile::tempdir().unwrap();
        let mut source = Db::open(&root.path().join("source.sqlite")).unwrap();
        let program = source
            .put_record(RecordInput {
                id: None,
                type_key: "program".into(),
                title: "MSc AI".into(),
                body: "A research program".into(),
                values: serde_json::json!({"level": "masters"}),
                tags: vec!["reach".into()],
                expected_revision: None,
            })
            .unwrap();
        let lab = source
            .put_record(RecordInput {
                id: None,
                type_key: "lab".into(),
                title: "Language Lab".into(),
                body: String::new(),
                values: serde_json::json!({}),
                tags: vec![],
                expected_revision: None,
            })
            .unwrap();
        let definition = source
            .put_relation_definition(RelationDefinitionInput {
                id: None,
                source_type: "program".into(),
                target_type: "lab".into(),
                name: "Research labs".into(),
                inverse_name: "Programs".into(),
            })
            .unwrap();
        source
            .put_view(TableViewInput {
                id: None,
                type_key: "program".into(),
                name: "Masters".into(),
                config: ViewConfig {
                    columns: vec!["title".into(), "level".into()],
                    filters: vec![FilterRule {
                        field: "level".into(),
                        operator: "equals".into(),
                        value: "masters".into(),
                    }],
                    filter_tree: None,
                    sorts: vec![SortRule {
                        field: "title".into(),
                        direction: "asc".into(),
                    }],
                    group_by: Some("level".into()),
                    column_widths: Default::default(),
                },
            })
            .unwrap();
        source
            .put_relation(RelationInput {
                source_id: program.id.clone(),
                target_id: lab.id.clone(),
                kind: String::new(),
                role: None,
                position: None,
                start_date: None,
                end_date: None,
                definition_id: Some(definition.id.clone()),
            })
            .unwrap();
        let export = root.path().join("export");
        assert_eq!(export_markdown(&source, &export).unwrap(), 2);
        let mut recovered = Db::open(&root.path().join("recovered.sqlite")).unwrap();
        recovered
            .put_record(RecordInput {
                id: None,
                type_key: "note".into(),
                title: "Existing note".into(),
                body: String::new(),
                values: serde_json::json!({}),
                tags: vec![],
                expected_revision: None,
            })
            .unwrap();
        let plan = scan(&export, &recovered.list_types().unwrap()).unwrap();
        assert_eq!(plan.preview.rows.len(), 2);
        assert!(plan.preview.blockers.is_empty());
        assert_eq!(apply(&mut recovered, plan).unwrap(), 2);
        assert_eq!(recovered.list_records(None, false).unwrap().len(), 3);
        assert_eq!(recovered.all_relations().unwrap().len(), 1);
        assert_eq!(
            recovered.all_relations().unwrap()[0]
                .definition_id
                .as_deref(),
            Some(definition.id.as_str())
        );
        assert_eq!(recovered.list_views("program").unwrap().len(), 2);
        assert_eq!(
            recovered.get_record(&program.id).unwrap().unwrap().tags,
            vec!["reach"]
        );
        let duplicate = scan(&export, &recovered.list_types().unwrap()).unwrap();
        assert!(apply(&mut recovered, duplicate).is_err());
        assert_eq!(recovered.list_records(None, false).unwrap().len(), 3);

        let stale = scan(&export, &source.list_types().unwrap()).unwrap();
        let path = export
            .join("records")
            .join("program")
            .join(format!("{}.md", program.id));
        fs::write(path, "changed").unwrap();
        let mut fresh = Db::open(&root.path().join("fresh.sqlite")).unwrap();
        assert!(apply(&mut fresh, stale).is_err());
        assert!(fresh.list_records(None, false).unwrap().is_empty());
    }

    #[test]
    fn legacy_import_preserves_originals_and_converts_links() {
        let root = tempfile::tempdir().unwrap();
        let legacy = root.path().join("legacy");
        fs::create_dir(&legacy).unwrap();
        fs::write(
            legacy.join("uni-A1.md"),
            "---\nname: Example University\ntags:\n  - eduport-type/university\ncountry: Turkey\n---\nAn old description.\n",
        )
        .unwrap();
        fs::write(
            legacy.join("lab-B2.md"),
            "---\nname: Speech Lab\ntags:\n  - eduport-type/lab\nuniversity: '[[uni-A1]]'\n---\nResearch notes.\n",
        )
        .unwrap();
        fs::create_dir(legacy.join("attachments")).unwrap();
        fs::write(legacy.join("attachments/report.pdf"), [0_u8, 1, 255]).unwrap();
        let mut db = Db::open(&root.path().join("data.sqlite")).unwrap();
        let plan = scan(&legacy, &db.list_types().unwrap()).unwrap();
        assert_eq!(plan.preview.rows.len(), 2);
        assert!(plan.preview.blockers.is_empty());
        assert_eq!(apply(&mut db, plan).unwrap(), 2);
        assert_eq!(db.list_relations("lab-B2").unwrap().len(), 1);
        assert!(
            db.all_files()
                .unwrap()
                .iter()
                .any(|(name, bytes)| name == "attachments/report.pdf" && bytes == &[0, 1, 255])
        );
        assert!(
            db.all_files()
                .unwrap()
                .iter()
                .any(|(name, _)| name == "legacy-original/uni-A1.md")
        );
    }
}
