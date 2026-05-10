//! On-disk store for entity records, layered over [`vaultdb_core::Vault`].
//!
//! Every entity — regardless of type — lives as a single `.md` file
//! directly at the vault root. Type is encoded in the file's
//! `eduport-type/<value>` tag, *not* in its folder location. This
//! matches the contract the Python sidecar (the pre-rewrite eduport
//! runtime) shipped: it walked `data_folder.glob("*.md")`
//! non-recursively and discriminated by tag.
//!
//! ## Why no per-type folders
//!
//! Earlier port iterations of this file introduced a `FolderMap`
//! that placed each entity type in its own subdirectory
//! (`universities/`, `labs/`, …). That convention does not appear
//! in the design spec and does not match any real vault on disk —
//! existing eduport users have all their entities at the root.
//! Reconcile that walked the per-type folders found zero entities
//! on a real vault because the convention was a fiction. The
//! current implementation honours the actual contract: root-flat
//! layout, tag-driven discrimination.
//!
//! Subfolders under the vault root (e.g. a user's own `notes/`
//! folder of Obsidian-style scratch notes) are intentionally
//! ignored — they're not entity files even if they end in `.md`.

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

/// Read- and write-side store for entity files. Wraps a
/// `vaultdb_core::Vault` and threads every mutation through
/// vaultdb-core's lock + atomic-write + journal machinery, so the
/// safety guarantees vaultdb ships with carry through unchanged.
pub struct EntityStore {
    vault: vaultdb_core::Vault,
}

impl EntityStore {
    pub fn new(vault: vaultdb_core::Vault) -> Self {
        Self { vault }
    }

    pub fn vault(&self) -> &vaultdb_core::Vault {
        &self.vault
    }

    /// List every entity of `kind`. Walks the vault root
    /// non-recursively, parses every `.md` file's frontmatter, and
    /// keeps the records whose `eduport-type/<value>` tag matches
    /// `kind`. Files whose tag designates a *different* type, or
    /// whose frontmatter doesn't parse cleanly, are silently
    /// skipped — the watcher / reconcile layer surfaces those as
    /// `parse_errors` on its own path.
    pub fn list_by_kind(&self, kind: EntityType) -> Result<Vec<Entity>, EntityStoreError> {
        let load = self
            .vault
            .load_records(&self.vault.root, false, false)
            .map_err(|e| EntityStoreError::Eduport(EduportError::Vaultdb(e)))?;
        let mut entities = Vec::with_capacity(load.records.len());
        for r in load.records {
            let yaml = match serde_yaml::to_string(&r.fields) {
                Ok(y) => y,
                Err(_) => continue, // skip records we can't re-serialise
            };
            match Entity::from_yaml(&yaml) {
                Ok(e) if e.entity_type() == kind => entities.push(e),
                // Wrong type or non-eduport frontmatter — skip
                // silently. The reconcile / watcher path is where
                // genuinely-invalid eduport files surface as
                // parse_errors; here we just filter.
                _ => {}
            }
        }
        Ok(entities)
    }

    /// Find a single entity by its filename stem (no `.md`). Looks
    /// at `<vault>/<name>.md`, parses, and verifies the type tag
    /// matches `kind`. Returns `Ok(None)` when the file is missing
    /// or its type disagrees with `kind`.
    pub fn find_by_name(
        &self,
        kind: EntityType,
        name: &str,
    ) -> Result<Option<Entity>, EntityStoreError> {
        let path = self.path_for(kind, name);
        if !path.exists() {
            return Ok(None);
        }
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| EntityStoreError::Eduport(EduportError::Io(e)))?;
        let yaml = match split_frontmatter(&raw) {
            Some((fm, _body)) => fm,
            None => return Ok(None),
        };
        let entity = Entity::from_yaml(yaml).map_err(|reason| EntityStoreError::ParseFailed {
            path: path.display().to_string(),
            reason,
        })?;
        Ok(if entity.entity_type() == kind {
            Some(entity)
        } else {
            None
        })
    }

    /// Compute the on-disk path where an entity with the given
    /// `name` (filename stem) lives. Entities are flat at the
    /// vault root; the `_kind` argument is preserved for signature
    /// compatibility with the rest of the crate (and as a hint to
    /// callers that the type is implicit in the file's tag, not
    /// its location).
    pub fn path_for(&self, _kind: EntityType, name: &str) -> PathBuf {
        self.vault.root.join(format!("{}.md", name))
    }

    // ── Write side ──────────────────────────────────────────────────

    /// Create a new entity file at `<vault>/<filename_stem>.md`.
    /// `entity` provides the frontmatter; `body` is the free-form
    /// notes section after the closing `---`. Errors if a file
    /// already exists at the target path — overwrite via
    /// [`Self::save`] instead.
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
        vaultdb_core::writer::atomic_write(&path, &content)
            .map_err(|e| EntityStoreError::Eduport(EduportError::Io(e)))?;
        Ok(path)
    }

    /// Overwrite an existing entity file in place, preserving the
    /// body (call [`Self::save_with_body`] to set both at once).
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
        let existing = std::fs::read_to_string(&path)
            .map_err(|e| EntityStoreError::Eduport(EduportError::Io(e)))?;
        let body = extract_body(&existing);
        let content = render_entity_file(entity, &body)?;
        vaultdb_core::writer::atomic_write(&path, &content)
            .map_err(|e| EntityStoreError::Eduport(EduportError::Io(e)))?;
        Ok(path)
    }

    /// Same as [`Self::save`] but takes the body explicitly. Use
    /// this when the body is changing alongside the frontmatter.
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
    /// Uses vaultdb-core's `DeleteBuilder` so the operation
    /// inherits the vault lock + atomic-rename infrastructure.
    ///
    /// The folder argument to `DeleteBuilder::new` is the empty
    /// string here — vaultdb interprets that as "scan the vault
    /// root non-recursively", which matches the root-flat layout.
    pub fn delete(
        &self,
        kind: EntityType,
        filename_stem: &str,
        permanent: bool,
    ) -> Result<(), EntityStoreError> {
        let filter = vaultdb_core::Expr::Predicate(vaultdb_core::Predicate::Equals {
            field: "_name".into(),
            value: vaultdb_core::Value::String(filename_stem.into()),
        });
        let builder = vaultdb_core::DeleteBuilder::new("", filter).permanent(permanent);
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
    /// across the vault that points at the old name. The folder
    /// argument is the empty string for the same root-flat reason
    /// as [`Self::delete`]; the wikilink rewrite still walks the
    /// whole vault recursively.
    pub fn rename_file(
        &self,
        _kind: EntityType,
        from: &str,
        to: &str,
    ) -> Result<(), EntityStoreError> {
        let report = vaultdb_core::RenameBuilder::new("", from, to)
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

/// Split a raw `---\nyaml\n---\nbody` file into `(yaml, body)`.
/// Returns `None` when the leading delimiter or its closing
/// counterpart is missing.
fn split_frontmatter(raw: &str) -> Option<(&str, &str)> {
    let trimmed = raw.strip_prefix("---\n")?;
    let close = trimmed.find("\n---\n")?;
    Some((&trimmed[..close], &trimmed[close + "\n---\n".len()..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_vault() -> (TempDir, vaultdb_core::Vault) {
        let dir = TempDir::new().unwrap();
        // Vaults are typically Obsidian-shaped (so we drop a stub
        // `.obsidian/` to satisfy any heuristic that cares), but no
        // per-type subfolders — entities go flat at the root.
        fs::create_dir(dir.path().join(".obsidian")).unwrap();
        let vault = vaultdb_core::Vault::with_root(dir.path().to_path_buf());
        (dir, vault)
    }

    #[test]
    fn list_by_kind_reads_universities_from_root() {
        let (dir, vault) = setup_vault();
        fs::write(
            dir.path().join("stanford.md"),
            "---\nname: Stanford\ntags:\n  - eduport-type/university\ncountry: USA\n---\nBody.\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("mit.md"),
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
    fn list_by_kind_filters_other_types() {
        let (dir, vault) = setup_vault();
        // One university, one note — both at root, distinguished
        // only by their type tag.
        fs::write(
            dir.path().join("stanford.md"),
            "---\nname: Stanford\ntags:\n  - eduport-type/university\ncountry: USA\n---\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("reading-list.md"),
            "---\nname: Reading list\ntags:\n  - eduport-type/note\n---\n",
        )
        .unwrap();

        let store = EntityStore::new(vault);
        let unis = store.list_by_kind(EntityType::University).unwrap();
        assert_eq!(unis.len(), 1);
        let notes = store.list_by_kind(EntityType::Note).unwrap();
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn list_by_kind_ignores_subfolder_markdown() {
        // Files in subfolders are user-managed Obsidian notes,
        // attachments, etc. — never entities. Walks must stay
        // non-recursive.
        let (dir, vault) = setup_vault();
        fs::create_dir(dir.path().join("notes")).unwrap();
        fs::write(
            dir.path().join("notes/stray.md"),
            "---\nname: Stray\ntags:\n  - eduport-type/note\n---\n",
        )
        .unwrap();
        let store = EntityStore::new(vault);
        assert_eq!(store.list_by_kind(EntityType::Note).unwrap().len(), 0);
    }

    #[test]
    fn find_by_name_returns_some_when_present() {
        let (dir, vault) = setup_vault();
        fs::write(
            dir.path().join("stanford.md"),
            "---\nname: Stanford\ntags:\n  - eduport-type/university\ncountry: USA\n---\n",
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
        // File exists at the root but its tag claims it's a Note.
        // Treated as not-found for "find a University".
        let (dir, vault) = setup_vault();
        fs::write(
            dir.path().join("wrong.md"),
            "---\nname: Wrong\ntags:\n  - eduport-type/note\n---\n",
        )
        .unwrap();
        let store = EntityStore::new(vault);
        let e = store.find_by_name(EntityType::University, "wrong").unwrap();
        assert!(e.is_none());
    }

    #[test]
    fn path_for_is_root_relative() {
        let (dir, vault) = setup_vault();
        let store = EntityStore::new(vault);
        let p = store.path_for(EntityType::University, "stanford-K9p3");
        assert_eq!(p, dir.path().join("stanford-K9p3.md"));
    }

    #[test]
    fn list_by_kind_handles_mixed_entity_types() {
        let (dir, vault) = setup_vault();
        fs::write(
            dir.path().join("stanford-app.md"),
            r#"---
name: Stanford CS PhD 2026
tags:
  - eduport-type/application
program: "[[Stanford CS PhD]]"
status: drafting
---
Body
"#,
        )
        .unwrap();
        let store = EntityStore::new(vault);
        let apps = store.list_by_kind(EntityType::Application).unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name(), "Stanford CS PhD 2026");
    }
}
