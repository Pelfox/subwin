use gpui::{IntoElement, ParentElement, Styled, div, prelude::FluentBuilder};
use gpui_component::StyledExt;

#[derive(Default, IntoElement)]
pub struct SettingsItem {
    label: &'static str,
    child: Option<gpui::AnyElement>,
}

impl SettingsItem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn label(mut self, label: &'static str) -> Self {
        self.label = label;
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }
}

impl gpui::RenderOnce for SettingsItem {
    fn render(self, _: &mut gpui::Window, _: &mut gpui::App) -> impl IntoElement {
        div()
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .child(div().child(self.label).font_semibold())
            .when(self.child.is_some(), |this| this.child(self.child.unwrap()))
    }
}
