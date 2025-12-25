//! Property tests for lockfile key formatting/parsing.

use proptest::prelude::*;

use calvin::domain::value_objects::{lockfile_key, parse_lockfile_key, LockfileNamespace};

fn relative_path_string() -> impl Strategy<Value = String> {
    // Cross-platform friendly "relative path" generator:
    // - no empty segments
    // - no ':' (avoids Windows drive/prefix edge cases)
    // - no leading '~' (handled by a separate strategy)
    //
    // We intentionally allow '.' and '-' and '_' to cover common filenames.
    let segment = proptest::string::string_regex("[A-Za-z0-9._-]{1,16}").unwrap();
    proptest::collection::vec(segment, 1..=4).prop_map(|segments| segments.join("/"))
}

fn tilde_path_string() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("~".to_string()),
        relative_path_string().prop_map(|suffix| format!("~/{}", suffix)),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        .. ProptestConfig::default()
    })]

    /// PROPERTY: `parse_lockfile_key(lockfile_key(ns, path))` always parses.
    #[test]
    fn property_lockfile_key_round_trips_for_relative_paths(
        rel in relative_path_string()
    ) {
        let path = std::path::Path::new(&rel);

        // Project namespace round-trip preserves namespace and path.
        let key = lockfile_key(LockfileNamespace::Project, path);
        let parsed = parse_lockfile_key(&key).map(|(ns, p)| (ns, p.to_string()));
        prop_assert_eq!(parsed, Some((LockfileNamespace::Project, rel.clone())));

        // Home namespace round-trip preserves namespace and prepends `~/`.
        let key = lockfile_key(LockfileNamespace::Home, path);
        let parsed = parse_lockfile_key(&key).map(|(ns, p)| (ns, p.to_string()));
        prop_assert_eq!(parsed, Some((LockfileNamespace::Home, format!("~/{}", rel))));
    }

    /// PROPERTY: Any `~` path always uses the `home:` prefix, regardless of namespace.
    #[test]
    fn property_tilde_paths_always_home_namespace(
        tilde_path in tilde_path_string()
    ) {
        let path = std::path::Path::new(&tilde_path);

        let project_key = lockfile_key(LockfileNamespace::Project, path);
        prop_assert!(project_key.starts_with("home:"), "Expected home: prefix, got {project_key}");

        let home_key = lockfile_key(LockfileNamespace::Home, path);
        prop_assert!(home_key.starts_with("home:"), "Expected home: prefix, got {home_key}");

        // Parsing always yields Home namespace for `home:` keys.
        let parsed = parse_lockfile_key(&project_key);
        prop_assert!(matches!(parsed, Some((LockfileNamespace::Home, _))));
    }

    /// PROPERTY: `parse_lockfile_key` never panics on arbitrary input.
    #[test]
    fn property_parse_lockfile_key_never_panics(
        s in ".{0,128}"
    ) {
        let _ = parse_lockfile_key(&s);
    }
}
