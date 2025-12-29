use gpui::{AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div};
use gpui_component::{
    StyledExt,
    group_box::{GroupBox, GroupBoxVariants},
    slider::{Slider, SliderEvent, SliderState},
    switch::Switch,
};

use crate::{components::settings_item::SettingsItem, entities::DataEntities};

pub struct SettingsPage {
    data: DataEntities,
    padding_from_button_state: Entity<SliderState>,
}

impl SettingsPage {
    pub fn new(data: &DataEntities, cx: &mut Context<Self>) -> Self {
        let config = {
            let settings_state = data.settings.read(cx);
            &settings_state.config.clone()
        };

        let padding_from_button_state = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .default_value(config.captions_config.padding_from_bottom as f32)
                .max(1024.0) // TODO: get an actual active window's height
        });
        let _ = cx.subscribe(
            &padding_from_button_state,
            |_, _, event: &SliderEvent, cx| match event {
                SliderEvent::Change(value) => {
                    println!("Value: {value:?}");
                    cx.notify();
                }
            },
        );

        Self {
            data: data.clone(),
            padding_from_button_state,
        }
    }
}

impl Render for SettingsPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let config = {
            let settings_state = self.data.settings.read(cx);
            &settings_state.config.clone()
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                GroupBox::new()
                    .outline()
                    .child(div().child("Транскрипция").text_xl().font_bold()) // TODO: use `title`
                    .child(
                        SettingsItem::new().label("Включить транскрипцию?").child(
                            Switch::new("enable_transcoding")
                                .on_click(cx.listener(|_, _, _, _| println!("Got event"))),
                        ),
                    )
                    .child(
                        SettingsItem::new()
                            .label("Включить автоматический перевод?")
                            .child(
                                Switch::new("enable_auto_translation")
                                    .checked(config.enable_auto_translation),
                            ),
                    ),
            )
            .child(
                GroupBox::new()
                    .outline()
                    .child(div().child("Внешний вид").text_xl().font_bold())
                    .child(
                        SettingsItem::new()
                            .label("Отступ от низа экрана")
                            .child(Slider::new(&self.padding_from_button_state).max_w_1_4()),
                    ),
            )
    }
}
