use super::*;
use std::path::PathBuf;

#[test]
fn layer_creation() {
    let path = LayerPath::new(
        PathBuf::from("~/.calvin/.promptpack"),
        PathBuf::from("/home/user/.calvin/.promptpack"),
    );
    let layer = Layer::new("user", path, LayerType::User);

    assert_eq!(layer.name, "user");
    assert_eq!(layer.layer_type, LayerType::User);
    assert_eq!(
        layer.path.original(),
        &PathBuf::from("~/.calvin/.promptpack")
    );
    assert_eq!(
        layer.path.resolved(),
        &PathBuf::from("/home/user/.calvin/.promptpack")
    );
    assert!(layer.assets.is_empty());
}
