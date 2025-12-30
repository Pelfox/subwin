use gpui::{Entity, ParentElement, Styled, div, hsla, px};
use gpui_component::{ActiveTheme, IconName, Sizable, StyledExt, button::Button};

use crate::entities::{CaptionsEntity, DataEntities};

pub struct CaptionsRootView {
    pub captions_entity: Entity<CaptionsEntity>,
}

impl CaptionsRootView {
    pub fn new(data: &DataEntities) -> Self {
        Self {
            captions_entity: data.captions.clone(),
        }
    }
}

impl gpui::Render for CaptionsRootView {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let state = self.captions_entity.read(cx);
        let caption_text = state.last_run_content.clone();
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_color(cx.theme().foreground)
            .child(
                div()
                    .w_full()
                    .max_w_5_6()
                    .px_4()
                    .py_2()
                    .rounded_xl()
                    .bg(hsla(0., 0., 0., 0.65))
                    .border_1()
                    .border_color(hsla(0., 0., 1., 0.18))
                    .shadow_lg()
                    .child(
                        div()
                            .text_2xl()
                            .font_semibold()
                            .text_center()
                            .text_color(hsla(0., 0., 1., 0.95))
                            .line_height(px(30.))
                            .line_clamp(2)
                            .overflow_hidden()
                            .child(caption_text),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .w_full()
                    .flex()
                    .p_2()
                    .items_start()
                    .justify_between()
                    .child(
                        Button::new("stop_transcribing")
                            .icon(IconName::Close)
                            .outline()
                            .small(),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(state.last_run_duration.to_string()),
                    ),
            )
    }
}
