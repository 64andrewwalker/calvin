//! File System Asset Repository
//!
//! Loads assets from the file system by parsing PromptPack files.

use crate::domain::entities::{Asset, AssetKind};
use crate::domain::ports::AssetRepository;
use crate::domain::value_objects::{Scope, Target};
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

/// Asset repository that loads from the file system
///
/// Parses `.md` files with YAML frontmatter from a PromptPack directory.
/// Uses the existing parser for now - will be refactored later.
pub struct FsAssetRepository;

impl FsAssetRepository {
    /// Create a new repository
    pub fn new() -> Self {
        Self
    }
}

impl Default for FsAssetRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl FsAssetRepository {
    /// Convert a legacy PromptAsset to domain Asset
    fn convert_prompt_asset(pa: crate::models::PromptAsset) -> Asset {
        let kind = match pa.frontmatter.kind {
            crate::models::AssetKind::Policy => AssetKind::Policy,
            crate::models::AssetKind::Action => AssetKind::Action,
            crate::models::AssetKind::Agent => AssetKind::Agent,
            crate::models::AssetKind::Skill => AssetKind::Skill,
        };

        let scope = match pa.frontmatter.scope {
            crate::models::Scope::Project => Scope::Project,
            crate::models::Scope::User => Scope::User,
        };

        let targets: Vec<Target> = pa
            .frontmatter
            .targets
            .iter()
            .map(|t| match t {
                crate::models::Target::ClaudeCode => Target::ClaudeCode,
                crate::models::Target::Cursor => Target::Cursor,
                crate::models::Target::VSCode => Target::VSCode,
                crate::models::Target::Antigravity => Target::Antigravity,
                crate::models::Target::Codex => Target::Codex,
                crate::models::Target::All => Target::All,
            })
            .collect();

        let mut asset = Asset::new(
            &pa.id,
            &pa.source_path,
            &pa.frontmatter.description,
            &pa.content,
        )
        .with_kind(kind)
        .with_scope(scope)
        .with_targets(targets);

        // Set apply pattern if present
        if let Some(apply) = &pa.frontmatter.apply {
            asset = asset.with_apply(apply.clone());
        }

        if !pa.frontmatter.allowed_tools.is_empty() {
            asset = asset.with_allowed_tools(pa.frontmatter.allowed_tools);
        }

        asset
    }

    fn load_skills(source: &Path) -> Result<Vec<Asset>> {
        let skills_root = source.join("skills");
        if !skills_root.exists() {
            return Ok(Vec::new());
        }

        if !skills_root.is_dir() {
            anyhow::bail!(
                "Expected 'skills' to be a directory: {}",
                skills_root.display()
            );
        }

        let mut skills = Vec::new();

        for entry in std::fs::read_dir(&skills_root)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            // Skip hidden directories inside skills/
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('.'))
            {
                continue;
            }

            let id = entry.file_name().to_string_lossy().to_string();
            let skill_md_path = path.join("SKILL.md");
            if !skill_md_path.is_file() {
                anyhow::bail!("Skill directory '{}' missing SKILL.md", id);
            }

            skills.push(Self::load_skill_dir(source, &path, &id)?);
        }

        skills.sort_by(|a, b| a.id().cmp(b.id()));
        Ok(skills)
    }

    fn load_skill_dir(source_root: &Path, skill_dir: &Path, id: &str) -> Result<Asset> {
        let skill_md_path = skill_dir.join("SKILL.md");
        let raw = std::fs::read_to_string(&skill_md_path)?;

        let extracted = crate::parser::extract_frontmatter(&raw, &skill_md_path)?;
        let mut frontmatter = crate::parser::parse_frontmatter(&extracted.yaml, &skill_md_path)?;

        // Skills do not support `apply` (semantic activation, not file-pattern matching).
        if frontmatter.apply.is_some() {
            anyhow::bail!(
                "Skill '{}' does not support 'apply' in frontmatter (remove the field)",
                id
            );
        }

        // `kind: skill` is optional for skills, but if present must be `skill`.
        if yaml_has_key(&extracted.yaml, "kind")
            && frontmatter.kind != crate::models::AssetKind::Skill
        {
            anyhow::bail!(
                "Skill '{}' frontmatter kind must be 'skill' (or omit it)",
                id
            );
        }
        frontmatter.kind = crate::models::AssetKind::Skill;

        // Make source_path relative to the layer root.
        let rel_source_path = skill_md_path
            .strip_prefix(source_root)
            .unwrap_or(&skill_md_path)
            .to_path_buf();

        let prompt_asset =
            crate::models::PromptAsset::new(id, rel_source_path, frontmatter, extracted.body);
        let mut asset = Asset::from(prompt_asset);

        let supplementals = Self::load_skill_supplementals(skill_dir)?;
        asset = asset.with_supplementals(supplementals);

        Ok(asset.with_kind(AssetKind::Skill))
    }

    fn load_skill_supplementals(skill_dir: &Path) -> Result<HashMap<std::path::PathBuf, String>> {
        let mut out = HashMap::new();
        Self::load_skill_supplementals_recursive(skill_dir, skill_dir, &mut out)?;
        Ok(out)
    }

    fn load_skill_supplementals_recursive(
        skill_root: &Path,
        current: &Path,
        out: &mut HashMap<std::path::PathBuf, String>,
    ) -> Result<()> {
        for entry in std::fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;

            // Security: do not follow symlinks.
            if file_type.is_symlink() {
                anyhow::bail!(
                    "Symlinks are not supported in skill directories: {}",
                    path.display()
                );
            }

            if file_type.is_dir() {
                // Skip hidden directories.
                if path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with('.'))
                {
                    continue;
                }
                Self::load_skill_supplementals_recursive(skill_root, &path, out)?;
                continue;
            }

            if !file_type.is_file() {
                continue;
            }

            if path.file_name() == Some(std::ffi::OsStr::new("SKILL.md")) {
                continue;
            }

            // Skip hidden files.
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('.'))
            {
                continue;
            }

            let rel = path.strip_prefix(skill_root).unwrap_or(&path).to_path_buf();
            let bytes = std::fs::read(&path)?;
            if is_binary(&bytes) {
                anyhow::bail!(
                    "Binary files are not supported in skills: {}",
                    rel.display()
                );
            }
            let content = String::from_utf8(bytes)
                .map_err(|_| anyhow::anyhow!("Invalid UTF-8 in skill file: {}", rel.display()))?;
            out.insert(rel, content);
        }
        Ok(())
    }
}

impl AssetRepository for FsAssetRepository {
    fn load_all(&self, source: &Path) -> Result<Vec<Asset>> {
        // Use the existing parser for now
        let prompt_assets = crate::parser::parse_directory(source)?;

        // Convert from PromptAsset to domain Asset
        let mut assets: Vec<Asset> = prompt_assets
            .into_iter()
            .map(Self::convert_prompt_asset)
            .collect();

        // Load skills (directory-based assets)
        assets.extend(Self::load_skills(source)?);

        Ok(assets)
    }

    fn load_by_path(&self, path: &Path) -> Result<Asset> {
        // Skill entrypoint: .../skills/<id>/SKILL.md
        if path.file_name() == Some(std::ffi::OsStr::new("SKILL.md")) {
            if let Some(skill_dir) = path.parent() {
                if skill_dir
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    == Some("skills")
                {
                    let id = skill_dir
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");
                    let source_root = skill_dir
                        .parent()
                        .and_then(|p| p.parent())
                        .unwrap_or_else(|| Path::new("."));
                    return Self::load_skill_dir(source_root, skill_dir, id);
                }
            }
        }

        // Use the existing parser for single-file assets
        let pa = crate::parser::parse_file(path)?;
        Ok(Self::convert_prompt_asset(pa))
    }
}

fn is_binary(content: &[u8]) -> bool {
    content.contains(&0)
}

fn yaml_has_key(yaml: &str, key: &str) -> bool {
    let Ok(value) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(yaml) else {
        return false;
    };
    let serde_yaml_ng::Value::Mapping(map) = value else {
        return false;
    };
    map.contains_key(serde_yaml_ng::Value::String(key.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_asset(dir: &Path, name: &str, content: &str) {
        let file = dir.join(format!("{}.md", name));
        std::fs::write(&file, content).unwrap();
    }

    #[test]
    fn load_all_from_empty_dir() {
        let dir = tempdir().unwrap();
        let repo = FsAssetRepository::new();

        let assets = repo.load_all(dir.path()).unwrap();

        assert!(assets.is_empty());
    }

    #[test]
    fn load_all_parses_assets() {
        let dir = tempdir().unwrap();
        create_test_asset(
            dir.path(),
            "test-policy",
            r#"---
description: A test policy
scope: project
---
# Policy Content

This is the policy body.
"#,
        );

        let repo = FsAssetRepository::new();
        let assets = repo.load_all(dir.path()).unwrap();

        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].id(), "test-policy");
        assert_eq!(assets[0].description(), "A test policy");
        assert_eq!(assets[0].scope(), Scope::Project);
    }

    #[test]
    fn load_by_path_parses_single_asset() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("single.md");
        std::fs::write(
            &file,
            r#"---
description: Single asset
scope: user
targets:
  - cursor
---
Content here
"#,
        )
        .unwrap();

        let repo = FsAssetRepository::new();
        let asset = repo.load_by_path(&file).unwrap();

        assert_eq!(asset.id(), "single");
        assert_eq!(asset.scope(), Scope::User);
        assert!(asset.targets().contains(&Target::Cursor));
    }

    #[test]
    fn test_load_skill_directory_valid() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("skills/my-skill/scripts")).unwrap();
        std::fs::write(
            dir.path().join("skills/my-skill/SKILL.md"),
            r#"---
description: My skill
scope: project
targets:
  - claude-code
allowed-tools:
  - git
---

# Instructions

Do the thing.
"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("skills/my-skill/reference.md"),
            "# Reference\n\nDetails.",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("skills/my-skill/scripts/validate.py"),
            "print('ok')\n",
        )
        .unwrap();

        let repo = FsAssetRepository::new();
        let assets = repo.load_all(dir.path()).unwrap();

        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].kind(), AssetKind::Skill);
        assert_eq!(assets[0].id(), "my-skill");
        assert_eq!(
            assets[0].source_path(),
            &std::path::PathBuf::from("skills/my-skill/SKILL.md")
        );
        assert_eq!(assets[0].allowed_tools(), &["git".to_string()]);
        assert!(assets[0]
            .supplementals()
            .contains_key(&std::path::PathBuf::from("reference.md")));
        assert!(assets[0]
            .supplementals()
            .contains_key(&std::path::PathBuf::from("scripts/validate.py")));
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_load_skill_directory_valid__with_empty_supplemental_file() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("skills/my-skill")).unwrap();
        std::fs::write(
            dir.path().join("skills/my-skill/SKILL.md"),
            r#"---
description: My skill
---
Body
"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("skills/my-skill/empty.md"), "").unwrap();

        let repo = FsAssetRepository::new();
        let assets = repo.load_all(dir.path()).unwrap();

        assert_eq!(assets.len(), 1);
        assert!(assets[0]
            .supplementals()
            .contains_key(&std::path::PathBuf::from("empty.md")));
        assert_eq!(
            assets[0]
                .supplementals()
                .get(&std::path::PathBuf::from("empty.md"))
                .unwrap(),
            ""
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_load_skill_directory_valid__skips_hidden_files() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("skills/my-skill")).unwrap();
        std::fs::write(
            dir.path().join("skills/my-skill/SKILL.md"),
            r#"---
description: My skill
---
Body
"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("skills/my-skill/.hidden.bin"), b"\0\x01").unwrap();

        let repo = FsAssetRepository::new();
        let assets = repo.load_all(dir.path()).unwrap();

        assert_eq!(assets.len(), 1);
        assert!(
            !assets[0]
                .supplementals()
                .contains_key(&std::path::PathBuf::from(".hidden.bin")),
            "expected hidden files to be skipped"
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_load_skill_directory_valid__skips_hidden_directories() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("skills/my-skill/.private")).unwrap();
        std::fs::write(
            dir.path().join("skills/my-skill/SKILL.md"),
            r#"---
description: My skill
---
Body
"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("skills/my-skill/.private/secret.md"),
            "secret",
        )
        .unwrap();

        let repo = FsAssetRepository::new();
        let assets = repo.load_all(dir.path()).unwrap();

        assert_eq!(assets.len(), 1);
        assert!(
            !assets[0]
                .supplementals()
                .contains_key(&std::path::PathBuf::from(".private/secret.md")),
            "expected hidden directories to be skipped"
        );
    }

    #[test]
    fn test_load_skill_directory_missing_skill_md_errors() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("skills/my-skill")).unwrap();
        std::fs::write(dir.path().join("skills/my-skill/reference.md"), "ref").unwrap();

        let repo = FsAssetRepository::new();
        let err = repo.load_all(dir.path()).unwrap_err();
        assert!(err.to_string().contains("missing SKILL.md"));
    }

    #[test]
    fn test_load_skill_directory_binary_supplemental_rejected() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("skills/my-skill")).unwrap();
        std::fs::write(
            dir.path().join("skills/my-skill/SKILL.md"),
            r#"---
description: My skill
---
Body
"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("skills/my-skill/binary.bin"), b"\0\x01\x02").unwrap();

        let repo = FsAssetRepository::new();
        let err = repo.load_all(dir.path()).unwrap_err();
        assert!(err.to_string().contains("Binary files are not supported"));
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_load_skill_directory_binary_supplemental_rejected__with_nul_in_text() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("skills/my-skill")).unwrap();
        std::fs::write(
            dir.path().join("skills/my-skill/SKILL.md"),
            r#"---
description: My skill
---
Body
"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("skills/my-skill/notes.md"), b"hello\0world").unwrap();

        let repo = FsAssetRepository::new();
        let err = repo.load_all(dir.path()).unwrap_err();
        assert!(err.to_string().contains("Binary files are not supported"));
        assert!(err.to_string().contains("notes.md"));
    }
}
