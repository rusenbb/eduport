//! On-disk store for entity records, layered over [`vaultdb_core::Vault`].
//!
//! Each entity type lives in its own folder under the vault root
//! (typically `<vault>/{universities,labs,people,programs,
//! applications,documents,emails,notes}/`). One `.md` file per record,
//! frontmatter holds the structured fields (parsed via
//! [`crate::entity::types::Entity::from_yaml`]), body is the
//! free-form notes section.
//!
//! Phase 6 ships read-side methods (`list_by_kind`, `find_by_name`)
//! and the YAML round-trip. Write-side mutations (create/update/
//! delete) flow through vaultdb-core's mutation builders so they
//! inherit the lock / atomic-write / journal infrastructure already
//! built in Phase A.

use std::path::PathBuf;

use crate::EduportError;
use crate::EntityType;
use crate::entity::types::Entity;

#[derive(Debug, thiserror::Error)]
pub enum EntityStoreError {
    #[error("entity {kind:?} not found: {name}")]
    NotFound { kind: EntityType, name: String },

    #[error("entity parse failed for {path}: {reason}")]
    ParseFailed { path: String, reason: String },

    #[error(transparent)]
    Eduport(#[from] EduportError),
}

impl From<EntityStoreError> for EduportError {
    fn from(e: EntityStoreError) -> Self {
        EduportError::Schema(e.to_string())
    }
}

/// Read-side store layered over a vaultdb-core `Vault`. Each entity
/// type maps to a folder under the vault root.
pub struct EntityStore {
    vault: vaultdb_core::Vault,
    folder_map: FolderMap,
}

/// Per-entity-type folder mapping. Defaults to the conventional
/// pluralised name for each entity type; consumers can override at
/// construction time when their settings.toml uses different folder
/// names.
#[derive(Debug, Clone)]
pub struct FolderMap {
    map: std::collections::HashMap<EntityType, String>,
}

impl Default for FolderMap {
    fn default() -> Self {
        let mut map = std::collections::HashMap::new();
        map.insert(EntityType::University, "universities".into());
        map.insert(EntityType::Lab, "labs".into());
        map.insert(EntityType::Person, "people".into());
        map.insert(EntityType::Program, "programs".into());
        map.insert(EntityType::Application, "applications".into());
        map.insert(EntityType::Document, "documents".into());
        map.insert(EntityType::Email, "emails".into());
        map.insert(EntityType::Note, "notes".into());
        Self { map }
    }
}

impl FolderMap {
    /// Override one folder name. Useful when settings declare a
    /// non-default `notes_folder` etc.
    pub fn with_folder(mut self, kind: EntityType, folder: impl Into<String>) -> Self {
        self.map.insert(kind, folder.into());
        self
    }

    pub fn folder_for(&self, kind: EntityType) -> &str {
        self.map
            .get(&kind)
            .expect("FolderMap missing entry; this can't happen because Default seeds all eight")
    }
}

impl EntityStore {
    pub fn new(vault: vaultdb_core::Vault) -> Self {
        Self {
            vault,
            folder_map: FolderMap::default(),
        }
    }

    pub fn with_folder_map(mut self, map: FolderMap) -> Self {
        self.folder_map = map;
        self
    }

    pub fn vault(&self) -> &vaultdb_core::Vault {
        &self.vault
    }

    pub fn folder_for(&self, kind: EntityType) -> &str {
        self.folder_map.folder_for(kind)
    }

    /// List every entity of the given kind. Reads frontmatter via
    /// vaultdb-core (no body content unless a per-record body is
    /// specifically needed by a downstream consumer).
    pub fn list_by_kind(&self, kind: EntityType) -> Result<Vec<Entity>, EntityStoreError> {
        let folder = self.folder_for(kind).to_string();
        let folder_path = self
            .vault
            .resolve_folder(&folder)
            .map_err(|e| EntityStoreError::Eduport(EduportError::Vaultdb(e)))?;
        // Use the eager load_records since we need all records.
        let load = self
            .vault
            .load_records(&folder_path, false, false)
            .map_err(|e| EntityStoreError::Eduport(EduportError::Vaultdb(e)))?;
        let mut entities = Vec::with_capacity(load.records.len());
        for r in load.records {
            let yaml =
                serde_yaml::to_string(&r.fields).map_err(|e| EntityStoreError::ParseFailed {
                    path: r.path.display().to_string(),
                    reason: format!("re-serialise frontmatter: {}", e),
                })?;
            match Entity::from_yaml(&yaml) {
                Ok(e) => {
                    if e.entity_type() == kind {
                        entities.push(e);
                    }
                    // If the file's tag-discriminator says a *different*
                    // type than the folder it lives in, we silently skip
                    // — matches the existing sidecar's behaviour.
                }
                Err(reason) => {
                    return Err(EntityStoreError::ParseFailed {
                        path: r.path.display().to_string(),
                        reason,
                    });
                }
            }
        }
        Ok(entities)
    }

    /// Find a single entity by its filename (without `.md`). The
    /// file's tag-discriminator must match `kind` — otherwise treated
    /// as not-found.
    pub fn find_by_name(
        &self,
        kind: EntityType,
        name: &str,
    ) -> Result<Option<Entity>, EntityStoreError> {
        let folder = self.folder_for(kind);
        let record = self
            .vault
            .find_by_name(folder, name)
            .map_err(|e| EntityStoreError::Eduport(EduportError::Vaultdb(e)))?;
        let Some(record) = record else {
            return Ok(None);
        };
        let yaml =
            serde_yaml::to_string(&record.fields).map_err(|e| EntityStoreError::ParseFailed {
                path: record.path.display().to_string(),
                reason: format!("re-serialise frontmatter: {}", e),
            })?;
        let entity = Entity::from_yaml(&yaml).map_err(|reason| EntityStoreError::ParseFailed {
            path: record.path.display().to_string(),
            reason,
        })?;
        Ok(if entity.entity_type() == kind {
            Some(entity)
        } else {
            None
        })
    }

    /// Compute the on-disk path where an entity with the given
    /// `name` (filename stem) would live for the given kind.
    pub fn path_for(&self, kind: EntityType, name: &str) -> PathBuf {
        self.vault
            .root
            .join(self.folder_for(kind))
            .join(format!("{}.md", name))
    }

    // ── Write side ──────────────────────────────────────────────────

    /// Create a new entity file at `<folder>/<filename_stem>.md`.
    /// `entity` provides the frontmatter; `body` is the free-form
    /// notes section after the closing `---`. Errors if a file
    /// already exists at the target path — overwrite via [`Self::save`]
    /// instead.
    pub fn create(
        &self,
        kind: EntityType,
        filename_stem: &str,
        entity: &Entity,
        body: &str,
    ) -> Result<PathBuf, EntityStoreError> {
        if entity.entity_type() != kind {
            return Err(EntityStoreError::Eduport(EduportError::Schema(format!(
                "kind/entity mismatch: store kind = {}, entity kind = {}",
                kind,
                entity.entity_type()
            ))));
        }
        let path = self.path_for(kind, filename_stem);
        if path.exists() {
            return Err(EntityStoreError::Eduport(EduportError::Schema(format!(
                "entity file already exists: {}",
                path.display()
            ))));
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| EntityStoreError::Eduport(EduportError::Io(e)))?;
        }
        let content = render_entity_file(entity, body)?;
        // Use vaultdb-core's atomic write so partial-write-on-crash is
        // impossible — same primitive every vaultdb mutation uses.
        vaultdb_core::writer::atomic_write(&path, &content)
            .map_err(|e| EntityStoreError::Eduport(EduportError::Io(e)))?;
        Ok(path)
    }

    /// Overwrite an existing entity file in place. The file's body is
    /// preserved (we only replace the frontmatter); call [`Self::save_with_body`]
    /// to set both at once. Errors if the file doesn't exist — create
    /// via [`Self::create`] instead.
    pub fn save(
        &self,
        kind: EntityType,
        filename_stem: &str,
        entity: &Entity,
    ) -> Result<PathBuf, EntityStoreError> {
        if entity.entity_type() != kind {
            return Err(EntityStoreError::Eduport(EduportError::Schema(format!(
                "kind/entity mismatch: store kind = {}, entity kind = {}",
                kind,
                entity.entity_type()
            ))));
        }
        let path = self.path_for(kind, filename_stem);
        if !path.exists() {
            return Err(EntityStoreError::NotFound {
                kind,
                name: filename_stem.into(),
            });
        }
        // Read existing file to preserve the body.
        let existing = std::fs::read_to_string(&path)
            .map_err(|e| EntityStoreError::Eduport(EduportError::Io(e)))?;
        let body = extract_body(&existing);
        let content = render_entity_file(entity, &body)?;
        vaultdb_core::writer::atomic_write(&path, &content)
            .map_err(|e| EntityStoreError::Eduport(EduportError::Io(e)))?;
        Ok(path)
    }

    /// Same as [`Self::save`] but takes the body explicitly. Use this
    /// when the body is changing alongside the frontmatter.
    pub fn save_with_body(
        &self,
        kind: EntityType,
        filename_stem: &str,
        entity: &Entity,
        body: &str,
    ) -> Result<PathBuf, EntityStoreError> {
        if entity.entity_type() != kind {
            return Err(EntityStoreError::Eduport(EduportError::Schema(format!(
                "kind/entity mismatch: store kind = {}, entity kind = {}",
                kind,
                entity.entity_type()
            ))));
        }
        let path = self.path_for(kind, filename_stem);
        if !path.exists() {
            return Err(EntityStoreError::NotFound {
                kind,
                name: filename_stem.into(),
            });
        }
        let content = render_entity_file(entity, body)?;
        vaultdb_core::writer::atomic_write(&path, &content)
            .map_err(|e| EntityStoreError::Eduport(EduportError::Io(e)))?;
        Ok(path)
    }

    /// Delete an entity. By default moves to `<vault>/.trash/`
    /// (collision-safe). Pass `permanent: true` to remove outright.
    /// Uses vaultdb-core's `DeleteBuilder` so it inherits the vault
    /// lock + atomic-rename + journal-recovery infrastructure.
    pub fn delete(
        &self,
        kind: EntityType,
        filename_stem: &str,
        permanent: bool,
    ) -> Result<(), EntityStoreError> {
        let folder = self.folder_for(kind).to_string();
        // Filter on virtual `_name` field so we target exactly this file.
        let filter = vaultdb_core::Expr::Predicate(vaultdb_core::Predicate::Equals {
            field: "_name".into(),
            value: vaultdb_core::Value::String(filename_stem.into()),
        });
        let builder = vaultdb_core::DeleteBuilder::new(folder, filter).permanent(permanent);
        let report = builder
            .execute(&self.vault)
            .map_err(|e| EntityStoreError::Eduport(EduportError::Vaultdb(e)))?;
        if report.changes.is_empty() {
            return Err(EntityStoreError::NotFound {
                kind,
                name: filename_stem.into(),
            });
        }
        if !report.errors.is_empty() {
            return Err(EntityStoreError::Eduport(EduportError::Schema(format!(
                "delete reported {} error(s); first: {}",
                report.errors.len(),
                report.errors[0].message
            ))));
        }
        Ok(())
    }

    /// Rename the file backing an entity, rewriting every wikilink
    /// across the vault that points at the old name. Uses vaultdb-
    /// core's `RenameBuilder` so the operation is journal-protected
    /// (a crash during the rewrite leaves a journal that the next
    /// mutation replays).
    ///
    /// `from` and `to` are filename stems (no `.md`).
    pub fn rename_file(
        &self,
        kind: EntityType,
        from: &str,
        to: &str,
    ) -> Result<(), EntityStoreError> {
        let folder = self.folder_for(kind).to_string();
        let report = vaultdb_core::RenameBuilder::new(folder, from, to)
            .execute(&self.vault)
            .map_err(|e| EntityStoreError::Eduport(EduportError::Vaultdb(e)))?;
        if !report.errors.is_empty() {
            return Err(EntityStoreError::Eduport(EduportError::Schema(format!(
                "rename reported {} error(s); first: {}",
                report.errors.len(),
                report.errors[0].message
            ))));
        }
        Ok(())
    }
}

/// Render an entity to its on-disk file contents:
/// `---\n<frontmatter yaml>---\n<body>\n`.
fn render_entity_file(entity: &Entity, body: &str) -> Result<String, EntityStoreError> {
    let yaml = entity.to_yaml().map_err(|reason| {
        EntityStoreError::Eduport(EduportError::Schema(format!(
            "render entity yaml: {}",
            reason
        )))
    })?;
    // `serde_yaml::to_string` ends with a trailing newline; we want a
    // clean `---\n<yaml>---\n<body>\n` shape. Trim and re-add.
    let yaml_trimmed = yaml.trim_end();
    let body_trimmed = body.trim_end_matches('\n');
    Ok(if body_trimmed.is_empty() {
        format!("---\n{}\n---\n", yaml_trimmed)
    } else {
        format!("---\n{}\n---\n{}\n", yaml_trimmed, body_trimmed)
    })
}

/// Extract the body (everything after the closing `---`) of an
/// existing entity file. Returns the empty string if no closing
/// delimiter is found — defensive for hand-edited files.
fn extract_body(content: &str) -> String {
    if let Some((_, body_start)) = vaultdb_core::frontmatter::extract_frontmatter(content) {
        content[body_start..].to_string()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_vault() -> (TempDir, vaultdb_core::Vault) {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".obsidian")).unwrap();
        for folder in [
            "universities",
            "labs",
            "people",
            "programs",
            "applications",
            "documents",
            "emails",
            "notes",
        ] {
            fs::create_dir(dir.path().join(folder)).unwrap();
        }
        let vault = vaultdb_core::Vault::with_root(dir.path().to_path_buf());
        (dir, vault)
    }

    #[test]
    fn list_by_kind_reads_universities() {
        let (dir, vault) = setup_vault();
        fs::write(
            dir.path().join("universities/stanford.md"),
            "---\nname: Stanford\ntags:\n  - eduport-type/university\ncountry: USA\n---\nBody.\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("universities/mit.md"),
            "---\nname: MIT\ntags:\n  - eduport-type/university\ncountry: USA\ncity: Cambridge\n---\nBody.\n",
        )
        .unwrap();

        let store = EntityStore::new(vault);
        let unis = store.list_by_kind(EntityType::University).unwrap();
        assert_eq!(unis.len(), 2);
        let names: Vec<&str> = unis.iter().map(|e| e.name()).collect();
        assert!(names.contains(&"Stanford"));
        assert!(names.contains(&"MIT"));
    }

    #[test]
    fn find_by_name_returns_some_when_present() {
        let (dir, vault) = setup_vault();
        fs::write(
            dir.path().join("universities/stanford.md"),
            "---\nname: Stanford\ntags:\n  - eduport-type/university\ncountry: USA\n---\nBody.\n",
        )
        .unwrap();
        let store = EntityStore::new(vault);
        let e = store
            .find_by_name(EntityType::University, "stanford")
            .unwrap();
        assert!(e.is_some());
        assert_eq!(e.unwrap().name(), "Stanford");
    }

    #[test]
    fn find_by_name_returns_none_when_absent() {
        let (_dir, vault) = setup_vault();
        let store = EntityStore::new(vault);
        let e = store
            .find_by_name(EntityType::University, "nonexistent")
            .unwrap();
        assert!(e.is_none());
    }

    #[test]
    fn find_by_name_returns_none_when_kind_mismatch() {
        // File exists in universities/ folder but its tag claims it's
        // a Note. Treated as not-found for "find a University".
        let (dir, vault) = setup_vault();
        fs::write(
            dir.path().join("universities/wrong.md"),
            "---\nname: Wrong\ntags:\n  - eduport-type/note\n---\nBody.\n",
        )
        .unwrap();
        let store = EntityStore::new(vault);
        let e = store.find_by_name(EntityType::University, "wrong").unwrap();
        assert!(e.is_none());
    }

    #[test]
    fn folder_map_overrides_default() {
        let (_dir, vault) = setup_vault();
        let store = EntityStore::new(vault)
            .with_folder_map(FolderMap::default().with_folder(EntityType::Note, "3-Notes"));
        assert_eq!(store.folder_for(EntityType::Note), "3-Notes");
    }

    #[test]
    fn list_by_kind_handles_mixed_entity_types() {
        // applications/ contains an Application record. It should
        // round-trip cleanly including the required `program` field.
        let (dir, vault) = setup_vault();
        fs::write(
            dir.path().join("applications/stanford-app.md"),
            r#"---
name: Stanford CS PhD 2026
tags:
  - eduport-type/application
program: "[[Stanford CS PhD]]"
status: drafting
---
Notes.
"#,
        )
        .unwrap();
        let store = EntityStore::new(vault);
        let apps = store.list_by_kind(EntityType::Application).unwrap();
        assert_eq!(apps.len(), 1);
        match &apps[0] {
            Entity::Application(a) => {
                assert_eq!(a.program.target, "Stanford CS PhD");
            }
            _ => panic!("expected Application"),
        }
    }

    fn make_university(name: &str) -> Entity {
        Entity::University(crate::entity::types::University {
            name: name.into(),
            tags: vec!["eduport-type/university".into()],
            country: "USA".into(),
            city: None,
            website: None,
            links: vec![],
            emails: vec![],
            custom: std::collections::BTreeMap::new(),
        })
    }

    #[test]
    fn create_writes_a_new_file_atomically() {
        let (dir, vault) = setup_vault();
        let store = EntityStore::new(vault);
        let entity = make_university("Stanford");
        let path = store
            .create(EntityType::University, "stanford", &entity, "Body notes.")
            .unwrap();
        assert_eq!(path, dir.path().join("universities/stanford.md"));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("---\n"));
        assert!(content.contains("name: Stanford"));
        assert!(content.contains("country: USA"));
        assert!(content.contains("Body notes."));
    }

    #[test]
    fn create_errors_when_file_already_exists() {
        let (dir, vault) = setup_vault();
        fs::write(
            dir.path().join("universities/x.md"),
            "---\nname: x\ntags:\n  - eduport-type/university\ncountry: USA\n---\n",
        )
        .unwrap();
        let store = EntityStore::new(vault);
        let entity = make_university("X");
        let result = store.create(EntityType::University, "x", &entity, "");
        assert!(matches!(
            result,
            Err(EntityStoreError::Eduport(EduportError::Schema(_)))
        ));
    }

    #[test]
    fn create_rejects_kind_entity_mismatch() {
        let (_dir, vault) = setup_vault();
        let store = EntityStore::new(vault);
        let university = make_university("X");
        // Try to create a "Note" using a University entity → mismatch.
        let result = store.create(EntityType::Note, "x", &university, "");
        assert!(matches!(
            result,
            Err(EntityStoreError::Eduport(EduportError::Schema(_)))
        ));
    }

    #[test]
    fn save_overwrites_frontmatter_preserving_body() {
        let (dir, vault) = setup_vault();
        let store = EntityStore::new(vault);
        let mut entity = make_university("Stanford");
        store
            .create(
                EntityType::University,
                "stanford",
                &entity,
                "Original body line.\n",
            )
            .unwrap();

        // Modify the entity and save. Body should be preserved.
        if let Entity::University(u) = &mut entity {
            u.city = Some("Stanford, CA".into());
        }
        store
            .save(EntityType::University, "stanford", &entity)
            .unwrap();

        let content = fs::read_to_string(dir.path().join("universities/stanford.md")).unwrap();
        assert!(content.contains("city: Stanford, CA"));
        assert!(content.contains("Original body line."));
    }

    #[test]
    fn save_with_body_replaces_both_frontmatter_and_body() {
        let (dir, vault) = setup_vault();
        let store = EntityStore::new(vault);
        let entity = make_university("Stanford");
        store
            .create(
                EntityType::University,
                "stanford",
                &entity,
                "Original body.",
            )
            .unwrap();
        store
            .save_with_body(
                EntityType::University,
                "stanford",
                &entity,
                "New body content.",
            )
            .unwrap();
        let content = fs::read_to_string(dir.path().join("universities/stanford.md")).unwrap();
        assert!(content.contains("New body content."));
        assert!(!content.contains("Original body."));
    }

    #[test]
    fn save_errors_when_file_missing() {
        let (_dir, vault) = setup_vault();
        let store = EntityStore::new(vault);
        let entity = make_university("Stanford");
        let result = store.save(EntityType::University, "ghost", &entity);
        assert!(matches!(result, Err(EntityStoreError::NotFound { .. })));
    }

    #[test]
    fn delete_moves_file_to_trash_by_default() {
        let (dir, vault) = setup_vault();
        let store = EntityStore::new(vault);
        let entity = make_university("Stanford");
        store
            .create(EntityType::University, "stanford", &entity, "")
            .unwrap();
        store
            .delete(EntityType::University, "stanford", false)
            .unwrap();

        assert!(!dir.path().join("universities/stanford.md").exists());
        assert!(dir.path().join(".trash/stanford.md").exists());
    }

    #[test]
    fn delete_permanent_removes_file_outright() {
        let (dir, vault) = setup_vault();
        let store = EntityStore::new(vault);
        let entity = make_university("Stanford");
        store
            .create(EntityType::University, "stanford", &entity, "")
            .unwrap();
        store
            .delete(EntityType::University, "stanford", true)
            .unwrap();
        assert!(!dir.path().join("universities/stanford.md").exists());
        assert!(!dir.path().join(".trash/stanford.md").exists());
    }

    #[test]
    fn delete_errors_when_file_missing() {
        let (_dir, vault) = setup_vault();
        let store = EntityStore::new(vault);
        let result = store.delete(EntityType::University, "ghost", false);
        assert!(matches!(result, Err(EntityStoreError::NotFound { .. })));
    }

    #[test]
    fn rename_file_renames_and_rewrites_backlinks() {
        // The killer feature: file rename + cross-vault wikilink rewrite,
        // protected by vaultdb-core's journal so a crash mid-rewrite is
        // recoverable.
        let (dir, vault) = setup_vault();
        let store = EntityStore::new(vault);
        let stanford = make_university("Stanford");
        store
            .create(EntityType::University, "stanford", &stanford, "")
            .unwrap();
        // A Lab that references the Stanford university by wikilink.
        let lab = Entity::Lab(crate::entity::types::Lab {
            name: "AI Lab".into(),
            tags: vec!["eduport-type/lab".into()],
            focus: None,
            website: None,
            university: Some(crate::wikilink::WikiLink::new("stanford")),
            links: vec![],
            emails: vec![],
            custom: std::collections::BTreeMap::new(),
        });
        store.create(EntityType::Lab, "ai-lab", &lab, "").unwrap();

        // Rename stanford → stanford-university; the Lab's wikilink
        // should follow.
        store
            .rename_file(EntityType::University, "stanford", "stanford-university")
            .unwrap();

        assert!(!dir.path().join("universities/stanford.md").exists());
        assert!(
            dir.path()
                .join("universities/stanford-university.md")
                .exists()
        );

        let lab_content = fs::read_to_string(dir.path().join("labs/ai-lab.md")).unwrap();
        assert!(
            lab_content.contains("[[stanford-university]]"),
            "expected backlink rewritten, got: {}",
            lab_content
        );
        assert!(!lab_content.contains("[[stanford]]"));
    }
}
