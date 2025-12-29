use gpui::{
    AppContext, Bounds, Context, Entity, IntoElement, ParentElement, Pixels, Point, Render,
    SharedString, Styled, TitlebarOptions, Window, WindowBounds, WindowOptions, div, px, size,
};
use gpui_component::{
    Disableable, IconName, IndexPath, StyledExt,
    button::Button,
    select::{Select, SelectEvent, SelectItem, SelectState},
};

use crate::{BackendBridge, entities::DataEntities, views::captions_root_view};

#[derive(Debug, Clone)]
struct AudioDevice {
    id: SharedString,
    visible_name: SharedString,
}

impl SelectItem for AudioDevice {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.visible_name.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }
}

pub struct OverviewPage {
    is_active: bool,
    active_audio_device: Entity<SelectState<Vec<AudioDevice>>>,
}

impl OverviewPage {
    pub fn new(data: &DataEntities, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let active_audio_device = cx.new(|cx| {
            let audio_devices_entity = data.audio_devices.read(cx);
            let devices: Vec<AudioDevice> = audio_devices_entity
                .audio_devices
                .iter()
                .map(|device| AudioDevice {
                    id: device.id.clone().into(),
                    visible_name: device.description.clone().into(),
                })
                .collect();

            let selected_audio_device = audio_devices_entity
                .audio_devices
                .iter()
                .position(|device| device.selected);

            SelectState::new(
                devices,
                selected_audio_device.map(IndexPath::new),
                window,
                cx,
            )
        });

        let audio_devices = data.audio_devices.clone();
        cx.observe_in(
            &audio_devices.clone(),
            window,
            move |this, _, window, cx| {
                let audio_devices = {
                    let state = &audio_devices.read(cx);
                    state.audio_devices.clone()
                };

                let devices = audio_devices
                    .iter()
                    .map(|device| AudioDevice {
                        id: device.id.clone().into(),
                        visible_name: device.description.clone().into(),
                    })
                    .collect::<Vec<_>>();

                this.active_audio_device.update(cx, |state, cx| {
                    state.set_items(devices, window, cx);
                });

                if let Some(selected_index) =
                    audio_devices.iter().position(|device| device.selected)
                {
                    this.active_audio_device.update(cx, |state, cx| {
                        state.set_selected_index(Some(IndexPath::new(selected_index)), window, cx);
                    });
                }
            },
        )
        .detach();

        cx.subscribe_in(
            &active_audio_device,
            window,
            |_, _, event, _, cx| match event {
                SelectEvent::Confirm(value) => {
                    let bridge = cx.global::<BackendBridge>().clone();
                    let selected_value = value.clone();
                    if selected_value.is_none() {
                        return;
                    }

                    let selected_value = selected_value
                        .expect("failed to get the selected value")
                        .clone()
                        .into();
                    cx.spawn(async move |_, _| {
                        bridge.select_audio_device(selected_value).await;
                    })
                    .detach();
                }
            },
        )
        .detach();

        Self {
            is_active: false,
            active_audio_device,
        }
    }
}

impl Render for OverviewPage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(div().child("Главная").text_2xl().font_bold())
            .child(Select::new(&self.active_audio_device).placeholder("Выберите источник звука..."))
            .child(
                div().flex().gap_3().child(
                    Button::new("start_transcription")
                        .disabled(self.is_active)
                        .label("Включить субтитры")
                        .on_click(|_, window, cx| {
                            let (window_width, window_height) = (px(800.0), px(80.0));

                            let display = window
                                .display(cx)
                                .expect("failed to get current window's display");

                            let display_size = display.bounds().size;
                            let origin = Point::new(
                                (display_size.width - window_width) / 2.0,
                                Pixels::from(display_size.height.to_f64() - 180.0),
                            );

                            let caption_window_bounds =
                                Bounds::new(origin, size(window_width, window_height));

                            let captions_window_options: WindowOptions = WindowOptions {
                                window_bounds: Some(WindowBounds::Windowed(caption_window_bounds)),
                                titlebar: None,
                                focus: false,
                                show: true,
                                kind: gpui::WindowKind::PopUp,
                                is_movable: true,
                                is_resizable: false,
                                is_minimizable: false,
                                display_id: None,
                                window_background: gpui::WindowBackgroundAppearance::Opaque,
                                app_id: Some("subwin".to_owned()),
                                window_min_size: None,
                                window_decorations: None,
                                tabbing_identifier: Some("subwin".to_owned()),
                            };

                            cx.open_window(captions_window_options, |_, cx| {
                                cx.new(|_| captions_root_view::CaptionsRootView)
                            })
                            .expect("failed to open captions window");

                            // let bridge = cx.
                        }),
                ),
            )
    }
}
