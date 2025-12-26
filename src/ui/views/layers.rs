use calvin::application::layers::LayerQueryResult;

use crate::ui::blocks::header::CommandHeader;
use crate::ui::blocks::summary::ResultSummary;
use crate::ui::primitives::icon::Icon;
use crate::ui::primitives::text::{display_with_tilde, truncate_middle, ColoredText};
use crate::ui::widgets::r#box::{Box, BoxStyle};

pub struct LayersView<'a> {
    project_root: &'a std::path::Path,
    result: &'a LayerQueryResult,
}

impl<'a> LayersView<'a> {
    pub fn new(project_root: &'a std::path::Path, result: &'a LayerQueryResult) -> Self {
        Self {
            project_root,
            result,
        }
    }

    pub fn render(&self, supports_color: bool, supports_unicode: bool) -> String {
        let mut out = String::new();

        let mut header = CommandHeader::new(Icon::Check, "Calvin Layers");
        header.add("Project", self.project_root.display().to_string());
        header.add("Layers", self.result.layers.len().to_string());
        out.push_str(&header.render(supports_color, supports_unicode));
        out.push('\n');

        if self.result.layers.is_empty() {
            out.push_str(&format!(
                "{} {}\n",
                Icon::Warning.colored(supports_color, supports_unicode),
                ColoredText::warning("No layers found.").render(supports_color)
            ));
            return out;
        }

        let mut b = Box::with_title("Layer Stack (highest priority first)").style(BoxStyle::Info);
        let total_layers = self.result.layers.len();
        for (idx, layer) in self.result.layers.iter().rev().enumerate() {
            // Number from highest to lowest (e.g., 2, 1 for 2 layers)
            let layer_num = total_layers - idx;

            let path = truncate_middle(&display_with_tilde(&layer.original_path), 40);
            b.add_line(format!(
                "{}. [{}] {:<42} {:>3} assets",
                layer_num, layer.layer_type, path, layer.asset_count
            ));
        }
        out.push_str(&b.render(supports_color, supports_unicode));
        out.push('\n');

        let mut summary = ResultSummary::success("Layers Resolved");
        summary.add_stat("merged assets", self.result.merged_asset_count);
        summary.add_stat("overridden", self.result.overridden_asset_count);
        out.push_str(&summary.render(supports_color, supports_unicode));

        out
    }
}
