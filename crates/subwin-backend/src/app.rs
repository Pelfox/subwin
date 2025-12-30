//! Application context and message dispatching utilities.
//!
//! The context contains the shared state and provides helpers for sending
//! responses and notifications back to the frontend bridge.

use std::sync::Arc;

use subwin_bridge::{MessageFromBackend, MessageToBackend};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::services;
use crate::state::SharedState;

/// Shared application context passed to services and message handlers.
pub(crate) struct AppContext {
    /// Mutable runtime application state shared across services.
    pub state: SharedState,
    /// Outbound channel to the frontend bridge.
    pub tx: Sender<MessageFromBackend>,
}

impl AppContext {
    /// Read and dispatch messages from the frontend bridge until it closes.
    pub async fn consume_bridge_messages(self: &Arc<Self>, mut rx: Receiver<MessageToBackend>) {
        while let Some(message) = rx.recv().await {
            log::debug!("Got a frontend message: {message:?}");
            self.dispatch_message(message).await;
        }
    }

    /// Dispatches the received message from frontend down to individual
    /// service handlers.
    async fn dispatch_message(self: &Arc<Self>, message: MessageToBackend) {
        match message {
            MessageToBackend::ConfigurationRequest => {
                services::config_service::handle_config_request(self.clone()).await;
            }
            MessageToBackend::DownloadModelRequest(model) => {
                services::model_service::handle_download_model_request(self.clone(), model).await;
            }
            MessageToBackend::AudioDevicesListRequest => {
                services::audio_service::handle_audio_devices_list_request(self.clone()).await;
            }
            MessageToBackend::SelectAudioDevice(id) => {
                services::audio_service::handle_audio_device_selection(self.clone(), id).await;
            }
            MessageToBackend::StartTranscriptionRequest => {
                services::transcription_service::handle_start_transcription_request(self.clone())
                    .await;
            }
        }
    }

    /// Send a message to the frontend bridge.
    pub async fn send(&self, message: MessageFromBackend) {
        self.tx
            .send(message)
            .await
            .expect("failed to send message to frontend");
    }

    /// Send message synchronously (blocking) to the frontend bridge.
    pub fn send_blocking(&self, message: MessageFromBackend) {
        self.tx
            .blocking_send(message)
            .expect("failed to blocking send message to frontend");
    }

    /// Send a notification message to the frontend bridge.
    pub async fn send_notification(
        &self,
        notification_type: subwin_bridge::notification::NotificationType,
        content: impl Into<String>,
    ) {
        self.send(MessageFromBackend::NotificationMessage(
            subwin_bridge::notification::NotificationMessage {
                notification_type,
                message: content.into(),
            },
        ))
        .await;
    }
}
