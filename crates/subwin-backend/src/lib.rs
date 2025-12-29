//! Backend entry point and bridge message dispatcher.
//!
//! This crate owns the backend runtime lifecycle, loads configuration/state,
//! and reacts to messages from the frontend bridge.

mod config;
mod services;
mod state;

use std::{sync::Arc, thread};

use subwin_bridge::{MessageFromBackend, MessageToBackend};
use tokio::sync::{
    RwLock,
    mpsc::{Receiver, Sender},
};

use crate::state::{SharedState, State};

/// Shared application context passed to services and message handlers.
pub struct AppContext {
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
        }
    }

    /// Send a message to the frontend bridge.
    pub async fn send(&self, message: MessageFromBackend) {
        self.tx
            .send(message)
            .await
            .expect("failed to send message to frontend");
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

/// Initialize backend state and start processing frontend messages.
async fn setup_backend(rx: Receiver<MessageToBackend>, tx: Sender<MessageFromBackend>) {
    let (config, cache_path) = crate::config::load_config()
        .await
        .expect("failed to load config");

    let request_client = reqwest::Client::new();
    let active_host = Arc::new(cpal::default_host()); // using default host for now
    let active_audio_device = match config.audio_device_config.selected_device_id {
        Some(ref device_id) => {
            subwin_audio::device::get_device_by_id(&active_host, device_id.to_string())
                .expect("failed to get active audio device")
        }
        None => None,
    };

    let state = Arc::new(RwLock::new(State {
        config,
        cache_path,
        request_client,
        active_host,
        active_audio_device: Arc::new(active_audio_device),
    }));

    let context = Arc::new(AppContext { state, tx });
    context.consume_bridge_messages(rx).await;
}

/// Spawn the backend runtime and begin processing bridge messages.
pub fn run(rx: Receiver<MessageToBackend>, tx: Sender<MessageFromBackend>) {
    thread::spawn(move || {
        // TODO: use multi-threaded runtime?
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        runtime.block_on(async { setup_backend(rx, tx).await });
    });
}
