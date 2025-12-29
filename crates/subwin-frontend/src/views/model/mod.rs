use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Styled, Window, div,
    prelude::FluentBuilder,
};
use gpui_component::{
    StyledExt,
    group_box::{GroupBox, GroupBoxVariants},
};

use crate::{
    components::settings_item::SettingsItem,
    entities::{DataEntities, settings_entity::SettingsEntity},
    views::model::download_view::DownloadModelView,
};

mod download_view;

#[derive(Debug, Clone)]
pub struct ModelPage {
    settings: Entity<SettingsEntity>,
    download_view: Entity<DownloadModelView>,
}

impl ModelPage {
    pub fn new(data: &DataEntities, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            settings: data.settings.clone(),
            download_view: cx.new(|cx| DownloadModelView::new(data, window, cx)),
        }
    }
}

impl gpui::Render for ModelPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = self.settings.read(cx);

        div().size_full().when_else(
            settings.config.active_model_path.is_none(),
            |this| this.child(self.download_view.clone()),
            |this| {
                let active_model_path = settings
                    .config
                    .active_model_path
                    .as_ref()
                    .unwrap()
                    .display()
                    .to_string();
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(div().child("Настройки модели").text_2xl().font_bold())
                        .child(
                            GroupBox::new().outline().child(
                                SettingsItem::new()
                                    .label("Активная модель")
                                    .child(active_model_path),
                            ),
                        ),
                )
                // TODO: add other fields
            },
        )
    }
}
