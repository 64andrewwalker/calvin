//! Property tests for layer resolution.

use proptest::prelude::*;

use calvin::domain::services::{LayerResolveError, LayerResolver};
use tempfile::TempDir;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        .. ProptestConfig::default()
    })]

    /// PROPERTY: Layer resolution never panics and returns a clear error when no layers exist.
    #[test]
    fn property_layer_resolution_never_panics(
        user_layer_exists in any::<bool>(),
        project_layer_exists in any::<bool>(),
        additional_exists in proptest::collection::vec(any::<bool>(), 0..=3),
        remote_mode in any::<bool>(),
        disable_project_layer in any::<bool>(),
    ) {
        let project_root = TempDir::new().unwrap();
        let home_dir = TempDir::new().unwrap();

        // Arrange layer directories under the isolated temp roots.
        let project_layer_path = project_root.path().join(".promptpack");
        if project_layer_exists {
            std::fs::create_dir_all(&project_layer_path).unwrap();
        }

        let user_layer_path = home_dir.path().join(".calvin/.promptpack");
        if user_layer_exists {
            std::fs::create_dir_all(&user_layer_path).unwrap();
        }

        let mut additional_paths = Vec::new();
        for (idx, exists) in additional_exists.iter().enumerate() {
            let path = project_root
                .path()
                .join(".promptpack-layers")
                .join(format!("additional-{}", idx));
            if *exists {
                std::fs::create_dir_all(&path).unwrap();
            }
            additional_paths.push(path);
        }

        let resolver = LayerResolver::new(project_root.path().to_path_buf())
            .with_user_layer_path(user_layer_path)
            .with_additional_layers(additional_paths)
            .with_remote_mode(remote_mode)
            .with_disable_project_layer(disable_project_layer);

        let result = resolver.resolve();

        let any_additional_exists = additional_exists.iter().any(|b| *b);

        let expected_ok = if remote_mode {
            // Remote mode ignores user/additional and always relies on project layer.
            project_layer_exists
        } else {
            let project_effective = project_layer_exists && !disable_project_layer;
            user_layer_exists || any_additional_exists || project_effective
        };

        match (expected_ok, result) {
            (true, Ok(resolution)) => {
                prop_assert!(!resolution.layers.is_empty(), "expected at least one resolved layer");
            }
            (false, Err(LayerResolveError::NoLayersFound)) => {
                // Expected: clear error instead of panic or obscure IO error.
            }
            (false, other) => {
                prop_assert!(
                    matches!(&other, Err(LayerResolveError::NoLayersFound)),
                    "expected NoLayersFound when no layers exist, got: {other:?}"
                );
            }
            (true, other) => {
                prop_assert!(
                    other.is_ok(),
                    "expected successful resolution, got: {other:?}"
                );
            }
        }
    }
}
