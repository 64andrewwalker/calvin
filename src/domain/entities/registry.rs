//! Registry entity
//!
//! Tracks all projects managed by Calvin for global operations like `calvin projects`
//! and `calvin clean --all`.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectEntry {
    pub path: PathBuf,
    pub lockfile: PathBuf,
    pub last_deployed: DateTime<Utc>,
    pub asset_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Registry {
    pub version: u32,
    pub projects: Vec<ProjectEntry>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            version: 1,
            projects: Vec::new(),
        }
    }

    pub fn upsert(&mut self, entry: ProjectEntry) {
        if let Some(existing) = self.projects.iter_mut().find(|p| p.path == entry.path) {
            *existing = entry;
        } else {
            self.projects.push(entry);
        }
    }

    pub fn remove(&mut self, path: &Path) -> bool {
        let len_before = self.projects.len();
        self.projects.retain(|p| p.path != path);
        self.projects.len() != len_before
    }

    pub fn prune(&mut self) -> Vec<PathBuf> {
        let (valid, invalid): (Vec<_>, Vec<_>) =
            self.projects.drain(..).partition(|p| p.lockfile.exists());

        let removed: Vec<_> = invalid.into_iter().map(|p| p.path).collect();
        self.projects = valid;
        removed
    }

    pub fn all(&self) -> &[ProjectEntry] {
        &self.projects
    }
}

#[cfg(test)]
mod tests;
