//! Property tests for path expansion and validation.

use proptest::prelude::*;

use calvin::domain::entities::Lockfile;
use calvin::domain::value_objects::SafePath;
use calvin::domain::value_objects::Scope;
use calvin::infrastructure::fs::expand_home;

fn relative_suffix() -> impl Strategy<Value = String> {
    let segment = proptest::string::string_regex("[A-Za-z0-9._-]{1,16}").unwrap();
    proptest::collection::vec(segment, 0..=4).prop_map(|segments| segments.join("/"))
}

fn non_tilde_path_string() -> impl Strategy<Value = String> {
    // Generate strings that are unlikely to be interpreted as tilde paths.
    // We keep these small to avoid OS/path corner cases.
    proptest::string::string_regex("[A-Za-z0-9./_-]{0,64}")
        .unwrap()
        .prop_filter("must not start with ~", |s| !s.starts_with('~'))
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        .. ProptestConfig::default()
    })]

    /// PROPERTY: Path normalization never panics on arbitrary input.
    ///
    /// This guards against panic regressions when normalizing paths for lockfile keys.
    #[test]
    fn property_normalize_never_panics(
        s in "(?s).{0,256}"
    ) {
        let _ = Lockfile::make_key(Scope::Project, &s);
        let _ = Lockfile::make_key(Scope::User, &s);
    }

    /// PROPERTY: `expand_home("~/...")` expands to `home_dir()/...` when HOME is known.
    #[test]
    fn property_tilde_round_trip(
        suffix in relative_suffix()
    ) {
        let tilde_path = if suffix.is_empty() {
            "~".to_string()
        } else {
            format!("~/{}", suffix)
        };

        let expanded = expand_home(std::path::Path::new(&tilde_path));

        if let Some(home) = dirs::home_dir() {
            let expected = if suffix.is_empty() { home } else { home.join(&suffix) };
            prop_assert_eq!(expanded, expected);
        } else {
            // If the home directory can't be determined, expansion is a no-op.
            prop_assert_eq!(expanded, std::path::PathBuf::from(&tilde_path));
        }
    }

    /// PROPERTY: `expand_home` is identity for paths that don't start with `~`.
    #[test]
    fn property_expand_home_non_tilde_is_identity(
        path in non_tilde_path_string()
    ) {
        let p = std::path::PathBuf::from(&path);
        prop_assert_eq!(expand_home(&p), p);
    }

    /// PROPERTY: `SafePath::new` never panics on arbitrary small strings.
    #[test]
    fn property_safe_path_new_never_panics(
        s in ".{0,128}"
    ) {
        let _ = SafePath::new(s);
    }
}
