use super::*;
use tempfile::tempdir;

#[test]
fn resolve_user_layer_only() {
    let dir = tempdir().unwrap();
    let user_layer = dir.path().join("user/.promptpack");
    std::fs::create_dir_all(&user_layer).unwrap();

    let resolver = LayerResolver::new(dir.path().to_path_buf()).with_user_layer_path(user_layer);

    let resolution = resolver.resolve().unwrap();
    assert_eq!(resolution.layers.len(), 1);
    assert_eq!(resolution.layers[0].layer_type, LayerType::User);
}

#[test]
fn resolve_all_layers_in_priority_order() {
    let dir = tempdir().unwrap();
    let user_layer = dir.path().join("user/.promptpack");
    let custom_layer = dir.path().join("custom/.promptpack");
    let project_root = dir.path().join("project");
    let project_layer = project_root.join(".promptpack");

    for layer in [&user_layer, &custom_layer, &project_layer] {
        std::fs::create_dir_all(layer).unwrap();
    }

    let resolver = LayerResolver::new(project_root)
        .with_user_layer_path(user_layer)
        .with_additional_layers(vec![custom_layer]);

    let resolution = resolver.resolve().unwrap();
    assert_eq!(resolution.layers.len(), 3);
    assert_eq!(resolution.layers[0].layer_type, LayerType::User);
    assert_eq!(resolution.layers[1].layer_type, LayerType::Custom);
    assert_eq!(resolution.layers[2].layer_type, LayerType::Project);
}

#[test]
fn remote_mode_uses_only_project_layer() {
    let dir = tempdir().unwrap();
    let project_root = dir.path().join("project");
    let project_layer = project_root.join(".promptpack");
    std::fs::create_dir_all(&project_layer).unwrap();

    let resolver = LayerResolver::new(project_root).with_remote_mode(true);
    let resolution = resolver.resolve().unwrap();
    assert_eq!(resolution.layers.len(), 1);
    assert_eq!(resolution.layers[0].layer_type, LayerType::Project);
}

#[test]
fn resolve_no_layers_errors() {
    let dir = tempdir().unwrap();
    let resolver = LayerResolver::new(dir.path().to_path_buf());
    assert_eq!(
        resolver.resolve().unwrap_err(),
        LayerResolveError::NoLayersFound
    );
}

#[test]
#[cfg(unix)]
fn resolve_symlink_layer() {
    let dir = tempdir().unwrap();
    let real_layer = dir.path().join("real/.promptpack");
    let symlink_layer = dir.path().join("link/.promptpack");

    std::fs::create_dir_all(&real_layer).unwrap();
    std::fs::create_dir_all(symlink_layer.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&real_layer, &symlink_layer).unwrap();

    let project_root = dir.path().join("project");
    std::fs::create_dir_all(project_root.join(".promptpack")).unwrap();

    let resolver = LayerResolver::new(project_root).with_additional_layers(vec![symlink_layer]);
    let resolution = resolver.resolve().unwrap();

    assert_eq!(resolution.layers.len(), 2);
    assert_eq!(resolution.layers[0].layer_type, LayerType::Custom);
    assert_eq!(
        resolution.layers[0].path.original(),
        &dir.path().join("link/.promptpack")
    );
    assert_eq!(
        resolution.layers[0].path.resolved(),
        &real_layer.canonicalize().unwrap()
    );
}

#[test]
#[cfg(unix)]
fn detect_circular_symlink() {
    let dir = tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");

    std::os::unix::fs::symlink(&b, &a).unwrap();
    std::os::unix::fs::symlink(&a, &b).unwrap();

    let project_root = dir.path().join("project");
    std::fs::create_dir_all(project_root.join(".promptpack")).unwrap();

    let resolver = LayerResolver::new(project_root).with_additional_layers(vec![a]);
    let err = resolver.resolve().unwrap_err();
    assert!(matches!(err, LayerResolveError::CircularSymlink { .. }));
}
