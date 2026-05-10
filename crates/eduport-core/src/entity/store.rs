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
}
