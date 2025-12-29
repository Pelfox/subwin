use gpui::{ParentElement, Styled, div, prelude::FluentBuilder};
use gpui_component::ActiveTheme;

use crate::formatting::{format_bytes, format_eta, format_speed};

#[derive(Debug, Clone)]
pub struct DownloadIndicator {
    download_progress_event: Option<crate::entities::download_entity::DownloadProgressEvent>,
}

impl DownloadIndicator {
    pub fn new(data: &crate::entities::DataEntities, cx: &mut gpui::Context<Self>) -> Self {
        cx.subscribe(&data.download, |this, _, event, cx| {
            this.download_progress_event = if event.downloaded_bytes != event.total_bytes {
                Some(*event)
            } else {
                None
            };

            cx.notify();
        })
        .detach();
        Self {
            download_progress_event: None,
        }
    }
}

impl gpui::Render for DownloadIndicator {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        div()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .when(self.download_progress_event.is_some(), |this| {
                let progress = self.download_progress_event.unwrap();
                this.child(format!(
                    "Скачано {} из {} ({}). Осталось {}.",
                    format_bytes(progress.downloaded_bytes),
                    format_bytes(progress.total_bytes),
                    format_speed(progress.speed),
                    format_eta(progress.remaining_time),
                ))
            })
    }
}
