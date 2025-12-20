//! Lockfile Namespace Value Object
//!
//! Identifies which deployment scope a lockfile entry belongs to.

use std::path::Path;

/// Lockfile key namespace.
///
/// This allows a single lockfile to track multiple deployment destinations
/// without key collisions (e.g., `.claude/settings.json` in project vs home).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockfileNamespace {
    /// Project-local deployment
    Project,
    /// User home directory deployment
    Home,
}

impl LockfileNamespace {
    /// Convert namespace to string for storage
    pub fn as_str(&self) -> &'static str {
        match self {
            LockfileNamespace::Project => "project",
            LockfileNamespace::Home => "home",
        }
    }

    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "project" => Some(LockfileNamespace::Project),
            "home" => Some(LockfileNamespace::Home),
            _ => None,
        }
    }
}

impl std::fmt::Display for LockfileNamespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Generate a lockfile key from namespace and path
///
/// Keys are formatted as `{namespace}:{path}` where namespace is "home" or "project".
///
/// Special handling:
/// - Paths starting with `~` always use `home:` prefix (regardless of namespace)
/// - Home namespace paths that don't start with `~` get `~/` prepended
pub fn lockfile_key(namespace: LockfileNamespace, path: &Path) -> String {
    let path_str = path.to_string_lossy();

    // Paths starting with ~ always use home: prefix
    if path_str == "~" || path_str.starts_with("~/") {
        return format!("home:{}", path_str);
    }

    match namespace {
        LockfileNamespace::Project => format!("project:{}", path_str),
        LockfileNamespace::Home => format!("home:~/{}", path_str),
    }
}

/// Parse a lockfile key into namespace and path
pub fn parse_lockfile_key(key: &str) -> Option<(LockfileNamespace, &str)> {
    if let Some(path) = key.strip_prefix("project:") {
        Some((LockfileNamespace::Project, path))
    } else if let Some(path) = key.strip_prefix("home:") {
        Some((LockfileNamespace::Home, path))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespace_as_str() {
        assert_eq!(LockfileNamespace::Project.as_str(), "project");
        assert_eq!(LockfileNamespace::Home.as_str(), "home");
    }

    #[test]
    fn namespace_parse() {
        assert_eq!(
            LockfileNamespace::parse("project"),
            Some(LockfileNamespace::Project)
        );
        assert_eq!(
            LockfileNamespace::parse("home"),
            Some(LockfileNamespace::Home)
        );
        assert_eq!(LockfileNamespace::parse("invalid"), None);
    }

    #[test]
    fn namespace_display() {
        assert_eq!(format!("{}", LockfileNamespace::Project), "project");
        assert_eq!(format!("{}", LockfileNamespace::Home), "home");
    }

    #[test]
    fn lockfile_key_project_path() {
        let key = lockfile_key(LockfileNamespace::Project, Path::new("file.md"));
        assert_eq!(key, "project:file.md");
    }

    #[test]
    fn lockfile_key_home_path() {
        let key = lockfile_key(LockfileNamespace::Home, Path::new("file.md"));
        assert_eq!(key, "home:~/file.md");
    }

    #[test]
    fn lockfile_key_tilde_path_always_home() {
        // Even with Project namespace, ~ paths use home: prefix
        let key = lockfile_key(LockfileNamespace::Project, Path::new("~/.config/test"));
        assert_eq!(key, "home:~/.config/test");
    }

    #[test]
    fn lockfile_key_bare_tilde() {
        let key = lockfile_key(LockfileNamespace::Home, Path::new("~"));
        assert_eq!(key, "home:~");
    }

    #[test]
    fn parse_lockfile_key_project() {
        let result = parse_lockfile_key("project:file.md");
        assert_eq!(result, Some((LockfileNamespace::Project, "file.md")));
    }

    #[test]
    fn parse_lockfile_key_home() {
        let result = parse_lockfile_key("home:~/.config/test");
        assert_eq!(result, Some((LockfileNamespace::Home, "~/.config/test")));
    }

    #[test]
    fn parse_lockfile_key_invalid() {
        assert_eq!(parse_lockfile_key("invalid:path"), None);
        assert_eq!(parse_lockfile_key("no-colon"), None);
    }
}
