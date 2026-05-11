//! On-disk store for user-saved views.
//!
//! Views live at `<vault>/.eduport/views.yaml`. Same atomicity model
//! as [`crate::schema::SchemaStore`]: in-memory cache, atomic save via
//! [`vaultdb_core::writer::atomic_write`], thread-safe via internal
//! mutex.

use std::path::PathBuf;
use std::sync::Mutex;

use crate::EduportError;
use crate::EntityType;
use crate::schema::store::ED_DIR_NAME;
use crate::view::types::{View, ViewsFile, empty_views_file};

pub const VIEWS_FILENAME: &str = "views.yaml";

#[derive(Debug, thiserror::Error)]
pub enum ViewStoreError {
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Eduport(#[from] EduportError),
}

impl From<ViewStoreError> for EduportError {
    fn from(e: ViewStoreError) -> Self {
        EduportError::Schema(e.to_string())
    }
}

pub struct ViewStore {
    data_folder: PathBuf,
    inner: Mutex<Option<ViewsFile>>,
}

impl ViewStore {
    pub fn new(data_folder: impl Into<PathBuf>) -> Self {
        Self {
            data_folder: data_folder.into(),
            inner: Mutex::new(None),
        }
    }

    pub fn views_dir(&self) -> PathBuf {
        self.data_folder.join(ED_DIR_NAME)
    }

    pub fn views_path(&self) -> PathBuf {
        self.views_dir().join(VIEWS_FILENAME)
    }

    pub fn load(&self) -> Result<ViewsFile, EduportError> {
        let mut guard = self.inner.lock().expect("ViewStore mutex poisoned");
        let v = self.load_locked()?;
        *guard = Some(v.clone());
        Ok(v)
    }

    pub fn current(&self) -> Result<ViewsFile, EduportError> {
        let mut guard = self.inner.lock().expect("ViewStore mutex poisoned");
        if let Some(v) = &*guard {
            return Ok(v.clone());
        }
        let v = self.load_locked()?;
        *guard = Some(v.clone());
        Ok(v)
    }

    pub fn reload(&self) -> Result<ViewsFile, EduportError> {
        let mut guard = self.inner.lock().expect("ViewStore mutex poisoned");
        *guard = None;
        let v = self.load_locked()?;
        *guard = Some(v.clone());
        Ok(v)
    }

    pub fn add_view(
        &self,
        entity_type: EntityType,
        view: View,
    ) -> Result<ViewsFile, ViewStoreError> {
        view.validate().map_err(ViewStoreError::Invalid)?;
        let mut guard = self.inner.lock().expect("ViewStore mutex poisoned");
        let mut file = match &*guard {
            Some(f) => f.clone(),
            None => self.load_locked()?,
        };
        let tv = file.types.entry(entity_type).or_default();
        if tv.view(&view.id).is_some() {
            return Err(ViewStoreError::Conflict(format!(
                "view {:?} already exists on {}",
                view.id, entity_type
            )));
        }
        tv.views.push(view);
        self.save_locked(&file)?;
        *guard = Some(file.clone());
        Ok(file)
    }

    pub fn update_view(
        &self,
        entity_type: EntityType,
        view: View,
    ) -> Result<ViewsFile, ViewStoreError> {
        view.validate().map_err(ViewStoreError::Invalid)?;
        let mut guard = self.inner.lock().expect("ViewStore mutex poisoned");
        let mut file = match &*guard {
            Some(f) => f.clone(),
            None => self.load_locked()?,
        };
        let tv = file.types.entry(entity_type).or_default();
        let pos = tv
            .views
            .iter()
            .position(|v| v.id == view.id)
            .ok_or_else(|| {
                ViewStoreError::NotFound(format!("no view {:?} on {}", view.id, entity_type))
            })?;
        tv.views[pos] = view;
        self.save_locked(&file)?;
        *guard = Some(file.clone());
        Ok(file)
    }

    pub fn delete_view(
        &self,
        entity_type: EntityType,
        view_id: &str,
    ) -> Result<ViewsFile, ViewStoreError> {
        let mut guard = self.inner.lock().expect("ViewStore mutex poisoned");
        let mut file = match &*guard {
            Some(f) => f.clone(),
            None => self.load_locked()?,
        };
        let tv = file.types.entry(entity_type).or_default();
        let before = tv.views.len();
        tv.views.retain(|v| v.id != view_id);
        if tv.views.len() == before {
            return Err(ViewStoreError::NotFound(format!(
                "no view {:?} on {}",
                view_id, entity_type
            )));
        }
        self.save_locked(&file)?;
        *guard = Some(file.clone());
        Ok(file)
    }

    /// Reorder a single entity-type's views to match the given id list.
    /// Errors if the id set differs from what's currently stored.
    pub fn reorder_views(
        &self,
        entity_type: EntityType,
        ordered_ids: &[String],
    ) -> Result<ViewsFile, ViewStoreError> {
        let mut guard = self.inner.lock().expect("ViewStore mutex poisoned");
        let mut file = match &*guard {
            Some(f) => f.clone(),
            None => self.load_locked()?,
        };
        let tv = file.types.entry(entity_type).or_default();
        let mut existing: std::collections::HashMap<String, View> =
            tv.views.iter().map(|v| (v.id.clone(), v.clone())).collect();
        let existing_ids: std::collections::HashSet<&String> = existing.keys().collect();
        let new_ids: std::collections::HashSet<&String> = ordered_ids.iter().collect();
        if existing_ids != new_ids {
            return Err(ViewStoreError::Invalid(
                "ordered_ids must contain exactly the existing view ids".into(),
            ));
        }
        let new_views: Vec<View> = ordered_ids
            .iter()
            .map(|id| existing.remove(id).unwrap())
            .collect();
        tv.views = new_views;
        self.save_locked(&file)?;
        *guard = Some(file.clone());
        Ok(file)
    }

    fn load_locked(&self) -> Result<ViewsFile, EduportError> {
        let path = self.views_path();
        if !path.exists() {
            let seeded = empty_views_file();
            self.save_locked(&seeded)?;
            return Ok(seeded);
        }
        let text = std::fs::read_to_string(&path).map_err(EduportError::Io)?;
        let file: ViewsFile = serde_yaml::from_str(&text)
            .map_err(|e| EduportError::Schema(format!("views.yaml: {}", e)))?;
        file.validate().map_err(EduportError::Schema)?;
        Ok(file)
    }

    fn save_locked(&self, file: &ViewsFile) -> Result<(), EduportError> {
        std::fs::create_dir_all(self.views_dir()).map_err(EduportError::Io)?;
        let text = serde_yaml::to_string(file)
            .map_err(|e| EduportError::Schema(format!("views serialize: {}", e)))?;
        vaultdb_core::writer::atomic_write(&self.views_path(), &text).map_err(EduportError::Io)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::types::{SortDir, ViewFilter, ViewKind};
    use tempfile::TempDir;

    fn store(dir: &TempDir) -> ViewStore {
        ViewStore::new(dir.path().to_path_buf())
    }

    fn make_view(id: &str) -> View {
        View {
            id: id.into(),
            name: format!("View {}", id),
            kind: ViewKind::List,
            filter: ViewFilter::default(),
            filter_tree: None,
            sort_key: None,
            sort_dir: SortDir::Asc,
            group_by_key: None,
            columns: None,
            card_properties: None,
        }
    }

    #[test]
    fn load_seeds_empty_views_file() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        let f = s.load().unwrap();
        assert_eq!(f.types.len(), 8);
        assert!(s.views_path().exists());
    }

    #[test]
    fn add_update_delete_round_trip() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        s.load().unwrap();
        let v = make_view("active");
        s.add_view(EntityType::Application, v.clone()).unwrap();

        let mut updated = v.clone();
        updated.name = "Active applications".into();
        s.update_view(EntityType::Application, updated.clone())
            .unwrap();

        let after = s.current().unwrap();
        assert_eq!(
            after.for_type(EntityType::Application).views[0].name,
            "Active applications"
        );

        s.delete_view(EntityType::Application, "active").unwrap();
        let after = s.current().unwrap();
        assert!(after.for_type(EntityType::Application).views.is_empty());
    }

    #[test]
    fn add_view_rejects_duplicate_id() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        s.load().unwrap();
        s.add_view(EntityType::Application, make_view("dup"))
            .unwrap();
        let r = s.add_view(EntityType::Application, make_view("dup"));
        assert!(matches!(r, Err(ViewStoreError::Conflict(_))));
    }

    #[test]
    fn update_view_errors_when_absent() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        s.load().unwrap();
        let r = s.update_view(EntityType::Application, make_view("ghost"));
        assert!(matches!(r, Err(ViewStoreError::NotFound(_))));
    }

    #[test]
    fn reorder_views_changes_order_persistently() {
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        s.load().unwrap();
        for id in ["a", "b", "c"] {
            s.add_view(EntityType::Application, make_view(id)).unwrap();
        }
        let after = s
            .reorder_views(
                EntityType::Application,
                &["c".to_string(), "a".into(), "b".into()],
            )
            .unwrap();
        let ids: Vec<&str> = after
            .for_type(EntityType::Application)
            .views
            .iter()
            .map(|v| v.id.as_str())
            .collect();
        assert_eq!(ids, vec!["c", "a", "b"]);
    }

    #[test]
    fn views_file_validates_on_load() {
        // Write a corrupt views.yaml manually and verify load() rejects it.
        let dir = TempDir::new().unwrap();
        let s = store(&dir);
        std::fs::create_dir_all(s.views_dir()).unwrap();
        std::fs::write(s.views_path(), "version: 99\ntypes: {}\n").unwrap();
        assert!(s.load().is_err());
    }
}
