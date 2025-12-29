use gpui::{ParentElement, Styled, div};
use gpui_component::{ActiveTheme, StyledExt};

pub struct CaptionsRootView;

impl gpui::Render for CaptionsRootView {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_xl()
                    .font_bold()
                    .text_color(cx.theme().foreground)
                    .child("Hello, world!"),
            )
    }
}
