use crate::ui::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderChar {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Horizontal,
    Vertical,
}

impl BorderChar {
    pub fn render(&self, supports_unicode: bool) -> &'static str {
        match (supports_unicode, self) {
            (true, BorderChar::TopLeft) => theme::borders::TOP_LEFT,
            (true, BorderChar::TopRight) => theme::borders::TOP_RIGHT,
            (true, BorderChar::BottomLeft) => theme::borders::BOTTOM_LEFT,
            (true, BorderChar::BottomRight) => theme::borders::BOTTOM_RIGHT,
            (true, BorderChar::Horizontal) => theme::borders::HORIZONTAL,
            (true, BorderChar::Vertical) => theme::borders::VERTICAL,
            (false, BorderChar::TopLeft) => theme::borders_ascii::TOP_LEFT,
            (false, BorderChar::TopRight) => theme::borders_ascii::TOP_RIGHT,
            (false, BorderChar::BottomLeft) => theme::borders_ascii::BOTTOM_LEFT,
            (false, BorderChar::BottomRight) => theme::borders_ascii::BOTTOM_RIGHT,
            (false, BorderChar::Horizontal) => theme::borders_ascii::HORIZONTAL,
            (false, BorderChar::Vertical) => theme::borders_ascii::VERTICAL,
        }
    }
}
