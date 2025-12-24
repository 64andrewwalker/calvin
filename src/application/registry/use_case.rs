//! Registry Use Case
//!
//! Application-layer orchestration for working with the global registry.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;

use crate::domain::entities::ProjectEntry;
use crate::domain::ports::{RegistryError, RegistryRepository};

pub struct RegistryUseCase {
    repository: Arc<dyn RegistryRepository>,
}

impl RegistryUseCase {
    pub fn new(repository: Arc<dyn RegistryRepository>) -> Self {
        Self { repository }
    }

    pub fn register_project(
        &self,
        project_path: &Path,
        lockfile_path: &Path,
        asset_count: usize,
    ) -> Result<(), RegistryError> {
        let entry = ProjectEntry {
            path: project_path.to_path_buf(),
            lockfile: lockfile_path.to_path_buf(),
            last_deployed: Utc::now(),
            asset_count,
        };
        self.repository.update_project(entry)
    }

    pub fn list_projects(&self) -> Vec<ProjectEntry> {
        self.repository.load().all().to_vec()
    }

    pub fn prune(&self) -> Result<Vec<PathBuf>, RegistryError> {
        let mut registry = self.repository.load();
        let removed = registry.prune();
        self.repository.save(&registry)?;
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::Registry;
    use std::sync::Mutex;

    struct InMemoryRegistryRepo {
        registry: Mutex<Registry>,
    }

    impl InMemoryRegistryRepo {
        fn new() -> Self {
            Self {
                registry: Mutex::new(Registry::new()),
            }
        }
    }

    impl RegistryRepository for InMemoryRegistryRepo {
        fn load(&self) -> Registry {
            self.registry.lock().unwrap().clone()
        }

        fn save(&self, registry: &Registry) -> Result<(), RegistryError> {
            *self.registry.lock().unwrap() = registry.clone();
            Ok(())
        }

        fn update_project(&self, entry: ProjectEntry) -> Result<(), RegistryError> {
            let mut reg = self.registry.lock().unwrap();
            reg.upsert(entry);
            Ok(())
        }
    }

    #[test]
    fn register_project_upserts() {
        let repo = Arc::new(InMemoryRegistryRepo::new());
        let use_case = RegistryUseCase::new(repo);

        use_case
            .register_project(Path::new("/p"), Path::new("/p/calvin.lock"), 1)
            .unwrap();
        use_case
            .register_project(Path::new("/p"), Path::new("/p/calvin.lock"), 2)
            .unwrap();

        let projects = use_case.list_projects();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].asset_count, 2);
    }
}
