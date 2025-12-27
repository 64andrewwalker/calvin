use super::*;
use crate::domain::entities::{Layer, LayerPath, LayerType};

fn layer_path(path: &str) -> LayerPath {
    LayerPath::new(PathBuf::from(path), PathBuf::from(path))
}

fn create_asset(id: &str, content: &str) -> Asset {
    Asset::new(id, format!("actions/{}.md", id), "desc", content)
}

fn create_skill_asset(id: &str, content: &str) -> Asset {
    Asset::new(id, format!("skills/{}/SKILL.md", id), "desc", content)
        .with_kind(crate::domain::entities::AssetKind::Skill)
}

#[test]
fn merge_same_id_higher_wins() {
    let user_layer = Layer::new("user", layer_path("user"), LayerType::User)
        .with_assets(vec![create_asset("style", "user content")]);

    let project_layer = Layer::new("project", layer_path("project"), LayerType::Project)
        .with_assets(vec![create_asset("style", "project content")]);

    let result = merge_layers(&[user_layer, project_layer]);

    assert_eq!(result.assets.len(), 1);
    let style = result.assets.get("style").unwrap();
    assert_eq!(style.source_layer, "project");
    assert_eq!(style.overrides, Some("user".to_string()));
}

#[test]
fn merge_different_ids_all_kept() {
    let user_layer = Layer::new("user", layer_path("user"), LayerType::User)
        .with_assets(vec![create_asset("security", "security rules")]);

    let project_layer = Layer::new("project", layer_path("project"), LayerType::Project)
        .with_assets(vec![create_asset("style", "style rules")]);

    let result = merge_layers(&[user_layer, project_layer]);

    assert_eq!(result.assets.len(), 2);
    assert!(result.assets.contains_key("security"));
    assert!(result.assets.contains_key("style"));
}

#[test]
fn merge_skill_id_does_not_conflict_with_non_skill_id() {
    let layer = Layer::new("project", layer_path("project"), LayerType::Project).with_assets(vec![
        create_asset("review", "action"),
        create_skill_asset("review", "skill"),
    ]);

    let result = merge_layers(&[layer]);

    assert_eq!(result.assets.len(), 2);
    assert!(result.assets.contains_key("review"));
    assert!(result.assets.contains_key("skill:review"));
}
