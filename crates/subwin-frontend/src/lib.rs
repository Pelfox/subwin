use gpui::{AppContext, Application, Global, WindowOptions};
use gpui_component::{
    Root, WindowExt,
    notification::{Notification, NotificationType},
};
use subwin_bridge::MessageFromBackend;
use tokio::sync::mpsc;

use crate::entities::{
    audio_devices_entity::AudioDevicesEntity,
    download_entity::{DownloadEntity, DownloadProgressEvent},
    settings_entity::SettingsEntity,
};

pub mod components;
pub mod entities;
pub mod formatting;
mod views;

#[derive(Clone)]
pub struct BackendBridge {
    pub to_backend: mpsc::Sender<subwin_bridge::MessageToBackend>,
}

impl BackendBridge {
    pub async fn request_config(&self) {
        self.to_backend
            .send(subwin_bridge::MessageToBackend::ConfigurationRequest)
            .await
            .expect("failed to request config");
    }

    pub async fn download_model(&self, model: subwin_bridge::whisper_model::WhisperModel) {
        self.to_backend
            .send(subwin_bridge::MessageToBackend::DownloadModelRequest(model))
            .await
            .expect("failed to request model download");
    }

    pub async fn request_audio_devices_list(&self) {
        self.to_backend
            .send(subwin_bridge::MessageToBackend::AudioDevicesListRequest)
            .await
            .expect("failed to request audio devices list");
    }

    pub async fn select_audio_device(&self, device_id: String) {
        self.to_backend
            .send(subwin_bridge::MessageToBackend::SelectAudioDevice(
                device_id,
            ))
            .await
            .expect("failed to select the audio device");
    }
}

impl Global for BackendBridge {}

pub fn run(
    mut rx: mpsc::Receiver<subwin_bridge::MessageFromBackend>,
    tx: mpsc::Sender<subwin_bridge::MessageToBackend>,
) -> anyhow::Result<()> {
    let app = Application::new().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);

        let download = cx.new(DownloadEntity::new);
        let settings = cx.new(|_| SettingsEntity::default());
        let audio_devices = cx.new(|_| AudioDevicesEntity::default());

        let data = entities::DataEntities {
            settings,
            download,
            audio_devices,
        };
        let listener_data = data.clone();

        let bridge = BackendBridge {
            to_backend: tx.clone(),
        };
        cx.set_global(bridge.clone());

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                // TODO: try to move this IPC handler to another place?
                let window_handle = window.window_handle();
                cx.spawn(async move |cx| {
                    while let Some(message) = rx.recv().await {
                        println!("Got a message from backend: {message:?}");
                        match message {
                            MessageFromBackend::ConfigurationResponse(config) => {
                                SettingsEntity::update(&listener_data.settings, config, cx)
                            }
                            MessageFromBackend::NotificationMessage(notification) => {
                                let _notification_type = match notification.notification_type {
                                    subwin_bridge::notification::NotificationType::Info => {
                                        NotificationType::Info
                                    }
                                    subwin_bridge::notification::NotificationType::Success => {
                                        NotificationType::Success
                                    }
                                    subwin_bridge::notification::NotificationType::Warning => {
                                        NotificationType::Warning
                                    }
                                    subwin_bridge::notification::NotificationType::Error => {
                                        NotificationType::Error
                                    }
                                };
                                window_handle
                                    .update(cx, |_, window, cx| {
                                        let _notification = Notification::new()
                                            .message(notification.message)
                                            .with_type(_notification_type);
                                        window.push_notification(_notification, cx);
                                    })
                                    .expect("failed to push a new notification");
                            }
                            MessageFromBackend::DownloadProgressUpdate {
                                downloaded_bytes,
                                total_bytes,
                                speed,
                                remaining_time,
                            } => {
                                // TODO: rewrite this to be like `SettingsEntity`?
                                let _ = listener_data.download.update(cx, |model, cx| {
                                    let event = DownloadProgressEvent {
                                        downloaded_bytes,
                                        total_bytes,
                                        speed,
                                        remaining_time,
                                    };
                                    model.progress = event;
                                    cx.emit(event);
                                    cx.notify();
                                });
                            }
                            MessageFromBackend::AudioDevicesListResponse(audio_devices) => {
                                let _ = listener_data.audio_devices.update(cx, |model, cx| {
                                    model.audio_devices = audio_devices;
                                    cx.notify();
                                });
                            }
                        }
                    }
                })
                .detach();

                // TODO: maybe move this into another place?
                cx.spawn(async move |_| {
                    bridge.request_config().await;
                    bridge.request_audio_devices_list().await;
                })
                .detach();

                let view = cx.new(|cx| crate::views::FrontendUi::new(&data, window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });

    Ok(())
}
