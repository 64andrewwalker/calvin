use crate::ui::primitives::icon::Icon;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ItemStatus {
    Pending,
    InProgress,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusItem {
    pub label: String,
    pub status: ItemStatus,
    pub detail: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct StatusList {
    items: Vec<StatusItem>,
    visible_count: Option<usize>,
    anchor: Option<usize>,
}

impl StatusList {
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn with_visible_count(count: usize) -> Self {
        Self {
            items: Vec::new(),
            visible_count: Some(count),
            anchor: None,
        }
    }

    pub fn add(&mut self, label: impl Into<String>) {
        self.items.push(StatusItem {
            label: label.into(),
            status: ItemStatus::Pending,
            detail: None,
        });
        self.anchor = Some(self.items.len().saturating_sub(1));
    }

    #[allow(dead_code)]
    pub fn set_anchor(&mut self, index: usize) {
        if index < self.items.len() {
            self.anchor = Some(index);
        }
    }

    pub fn update(&mut self, index: usize, status: ItemStatus) {
        if let Some(item) = self.items.get_mut(index) {
            item.status = status;
            self.anchor = Some(index);
        }
    }

    pub fn update_detail(&mut self, index: usize, detail: impl Into<String>) {
        if let Some(item) = self.items.get_mut(index) {
            item.detail = Some(detail.into());
            self.anchor = Some(index);
        }
    }

    pub fn render(&self, supports_color: bool, supports_unicode: bool) -> String {
        let visible: &[StatusItem] = match self.visible_count {
            None => &self.items,
            Some(count) if self.items.len() <= count => &self.items,
            Some(count) => {
                let anchor = self
                    .anchor
                    .unwrap_or_else(|| self.items.len().saturating_sub(1));
                let end = (anchor + 1).min(self.items.len());
                let start = end.saturating_sub(count);
                &self.items[start..end]
            }
        };

        let mut out = String::new();
        for item in visible {
            let icon = match item.status {
                ItemStatus::Pending => Icon::Pending,
                ItemStatus::InProgress => Icon::Progress,
                ItemStatus::Success => Icon::Success,
                ItemStatus::Warning => Icon::Warning,
                ItemStatus::Error => Icon::Error,
            }
            .colored(supports_color, supports_unicode);

            out.push_str("  ");
            out.push_str(&icon);
            out.push(' ');
            out.push_str(&item.label);
            if let Some(detail) = &item.detail {
                out.push_str("  ");
                out.push_str(detail);
            }
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_shows_pending_by_default() {
        let mut list = StatusList::default();
        list.add("a.md");
        let rendered = list.render(false, false);
        assert!(rendered.contains("[ ] a.md"));
    }

    #[test]
    fn update_changes_rendered_icon() {
        let mut list = StatusList::default();
        list.add("a.md");
        list.update(0, ItemStatus::Success);
        let rendered = list.render(false, false);
        assert!(rendered.contains("[OK] a.md"));
    }

    #[test]
    fn visible_count_shows_last_items() {
        let mut list = StatusList::with_visible_count(2);
        list.add("a");
        list.add("b");
        list.add("c");
        let rendered = list.render(false, false);
        assert!(!rendered.contains("a\n"));
        assert!(rendered.contains("b\n"));
        assert!(rendered.contains("c\n"));
    }
}
