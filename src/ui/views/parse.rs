use calvin::models::PromptAsset;

use crate::ui::widgets::r#box::{Box, BoxStyle};

pub fn render_parse_header(
    source: &str,
    asset_count: usize,
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let mut b = Box::with_style(BoxStyle::Info);
    b.add_line("Calvin Parse");
    b.add_line(format!("Source: {}", source));
    b.add_line(format!("Assets: {}", asset_count));
    b.render(supports_color, supports_unicode)
}

pub fn render_asset(asset: &PromptAsset, supports_color: bool, supports_unicode: bool) -> String {
    let mut b = Box::with_style(BoxStyle::Info);
    b.add_line(asset.id.as_str());
    b.add_line(format!("Description: {}", asset.frontmatter.description));
    b.add_line(format!("Kind: {:?}", asset.frontmatter.kind));
    b.add_line(format!("Scope: {:?}", asset.frontmatter.scope));
    b.add_line(format!("Path: {}", asset.source_path.display()));
    if !asset.frontmatter.targets.is_empty() {
        b.add_line(format!("Targets: {:?}", asset.frontmatter.targets));
    }
    if let Some(apply) = &asset.frontmatter.apply {
        b.add_line(format!("Apply: {}", apply));
    }
    b.render(supports_color, supports_unicode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use calvin::models::{AssetKind, Frontmatter, PromptAsset, Scope};

    #[test]
    fn parse_header_uses_themed_borders() {
        let rendered = render_parse_header(".promptpack", 1, false, true);
        assert!(rendered.starts_with('╭'));
    }

    #[test]
    fn asset_render_uses_themed_borders() {
        let mut fm = Frontmatter::new("Test");
        fm.kind = AssetKind::Action;
        fm.scope = Scope::Project;
        let asset = PromptAsset::new("id", "actions/id.md", fm, "Body");
        let rendered = render_asset(&asset, false, true);
        assert!(rendered.starts_with('╭'));
        assert!(rendered.contains("Description: Test"));
    }
}
