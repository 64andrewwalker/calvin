use calvin::domain::entities::Lockfile;

use crate::ui::blocks::header::CommandHeader;
use crate::ui::blocks::summary::ResultSummary;
use crate::ui::primitives::icon::Icon;
use crate::ui::primitives::text::ColoredText;
use crate::ui::widgets::r#box::{Box, BoxStyle};

pub struct ProvenanceView<'a> {
    lockfile_path: &'a std::path::Path,
    lockfile: &'a Lockfile,
    filter: Option<&'a str>,
}

impl<'a> ProvenanceView<'a> {
    pub fn new(
        lockfile_path: &'a std::path::Path,
        lockfile: &'a Lockfile,
        filter: Option<&'a str>,
    ) -> Self {
        Self {
            lockfile_path,
            lockfile,
            filter,
        }
    }

    pub fn render(&self, supports_color: bool, supports_unicode: bool) -> String {
        let mut out = String::new();

        let mut header = CommandHeader::new(Icon::Check, "Calvin Provenance");
        header.add("Lockfile", self.lockfile_path.display().to_string());
        if let Some(filter) = self.filter {
            header.add("Filter", filter.to_string());
        }
        out.push_str(&header.render(supports_color, supports_unicode));
        out.push('\n');

        let entries: Vec<_> = self
            .lockfile
            .entries()
            .filter(|(k, _)| self.filter.is_none_or(|f| k.contains(f)))
            .collect();

        if entries.is_empty() {
            out.push_str(&format!(
                "{} {}\n",
                Icon::Pending.colored(supports_color, supports_unicode),
                ColoredText::dim("No outputs found.").render(supports_color)
            ));
            return out;
        }

        let mut b = Box::with_title("Outputs").style(BoxStyle::Info);
        for (key, entry) in entries {
            let (_, path) = Lockfile::parse_key(key)
                .unwrap_or((calvin::domain::value_objects::Scope::Project, key));
            b.add_line(ColoredText::info(path).render(supports_color));
            if let Some(layer) = entry.source_layer() {
                b.add_line(format!(
                    "  {} {}",
                    Icon::Arrow.colored(supports_color, supports_unicode),
                    ColoredText::dim(format!("layer: {layer}")).render(supports_color)
                ));
            }
            if let Some(asset) = entry.source_asset() {
                b.add_line(format!(
                    "  {} {}",
                    Icon::Arrow.colored(supports_color, supports_unicode),
                    ColoredText::dim(format!("asset: {asset}")).render(supports_color)
                ));
            }
            if let Some(file) = entry.source_file() {
                b.add_line(format!(
                    "  {} {}",
                    Icon::Arrow.colored(supports_color, supports_unicode),
                    ColoredText::dim(format!("file: {}", file.display())).render(supports_color)
                ));
            }
            if let Some(overrides) = entry.overrides() {
                b.add_line(format!(
                    "  {} {}",
                    Icon::Warning.colored(supports_color, supports_unicode),
                    ColoredText::warning(format!("overrides: {overrides}")).render(supports_color)
                ));
            }
            b.add_empty();
        }

        out.push_str(&b.render(supports_color, supports_unicode));
        out.push('\n');

        let mut summary = ResultSummary::success("Provenance Loaded");
        summary.add_stat("outputs", self.lockfile.len());
        out.push_str(&summary.render(supports_color, supports_unicode));

        out
    }
}
