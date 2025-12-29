use gpui::{AppContext, Context, Entity, ParentElement, SharedString, Styled, Window, div};
use gpui_component::{
    IconName, IndexPath, StyledExt,
    button::{Button, ButtonVariants},
    select::{Select, SelectItem, SelectState},
};
use subwin_bridge::whisper_model::WhisperModel;

use crate::components::download_indicator::DownloadIndicator;

#[derive(Debug, Clone)]
struct Model {
    display_name: SharedString,
    value: WhisperModel,
}

impl Model {
    pub fn new(display_name: &'static str, value: WhisperModel) -> Self {
        Self {
            display_name: display_name.into(),
            value,
        }
    }
}

impl SelectItem for Model {
    type Value = WhisperModel;

    fn title(&self) -> SharedString {
        self.display_name.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }
}

#[derive(Debug, Clone)]
pub struct DownloadModelView {
    is_loading: bool,
    indicator: Entity<DownloadIndicator>,
    model_selector: Entity<SelectState<Vec<Model>>>,
}

impl DownloadModelView {
    pub fn new(
        data: &crate::entities::DataEntities,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let model_selector = cx.new(|cx| {
            let models: Vec<Model> = vec![
                // tiny
                Model::new("Мини (ускоренная, 8 бит)", WhisperModel::TinyQuantized8),
                Model::new("Мини (ускоренная, 5 бит)", WhisperModel::TinyQuantized5),
                Model::new("Мини", WhisperModel::Tiny),
                // small
                Model::new("Малая (ускоренная, 8 бит)", WhisperModel::SmallQuantized8),
                Model::new("Малая (ускоренная, 5 бит)", WhisperModel::SmallQuantized5),
                Model::new("Малая", WhisperModel::Small),
                // base
                Model::new("Базовая (ускоренная, 8 бит)", WhisperModel::BaseQuantized8),
                Model::new("Базовая (ускоренная, 5 бит)", WhisperModel::BaseQuantized5),
                Model::new("Базовая", WhisperModel::Base),
                // medium
                Model::new(
                    "Средняя (ускоренная, 8 бит)",
                    WhisperModel::MediumQuantized8,
                ),
                Model::new(
                    "Средняя (ускоренная, 5 бит)",
                    WhisperModel::MediumQuantized5,
                ),
                Model::new("Средняя", WhisperModel::Medium),
                // large
                Model::new(
                    "Большая турбо (ускоренная, 8 бит)",
                    WhisperModel::LargeTurboQuantized8,
                ),
                Model::new(
                    "Большая турбо (ускоренная, 5 бит)",
                    WhisperModel::LargeTurboQuantized5,
                ),
                Model::new("Большая турбо", WhisperModel::LargeTurbo),
                Model::new("Большая (ускоренная, 5 бит)", WhisperModel::LargeQuantized5),
                Model::new("Большая", WhisperModel::Large),
            ];
            SelectState::new(models, Some(IndexPath::default()), window, cx)
        });
        let indicator = cx.new(|cx| DownloadIndicator::new(data, cx));
        Self {
            is_loading: false,
            indicator,
            model_selector,
        }
    }
}

impl gpui::Render for DownloadModelView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .child(div().child("Скачайте модель").text_xl().font_bold())
            .child(div().child("Чтобы запустить приложение, сначала скачайте модель распознания."))
            .child(
                div()
                    .my_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        Select::new(&self.model_selector)
                            .disabled(self.is_loading)
                            .min_w_72()
                            .placeholder("Выберите модель..."),
                    )
                    .child(
                        Button::new("download_model")
                            .primary()
                            .icon(IconName::ArrowDown)
                            .loading(self.is_loading)
                            .label("Начать загрузку")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let bridge = cx.global::<crate::BackendBridge>().clone();
                                let selected_model =
                                    this.model_selector.read(cx).selected_value().cloned();

                                // TODO: should we display a notification error, if nothing has been selected?
                                if let Some(model) = selected_model {
                                    this.is_loading = true;
                                    cx.notify();
                                    cx.spawn(async move |_, _| {
                                        bridge.download_model(model).await;
                                    })
                                    .detach();
                                }
                            })),
                    ),
            )
            .child(self.indicator.clone())
    }
}
