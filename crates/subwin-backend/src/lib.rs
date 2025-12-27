mod config;
mod state;

mod services;

use std::{sync::Arc, thread};

use log::info;
use subwin_bridge::{MessageFromBackend, MessageToBackend};
use tokio::sync::{
    RwLock,
    mpsc::{Receiver, Sender},
};

use crate::state::{SharedState, State};

#[derive(Debug, Clone)]
pub struct AppContext {
    pub state: SharedState,
    pub tx: Sender<MessageFromBackend>,
}

impl AppContext {
    pub async fn consume_bridge_messages(&self, mut rx: Receiver<MessageToBackend>) {
        while let Some(message) = rx.recv().await {
            info!("Got a frontend message: {message:?}");
            self.dispatch_message(message).await;
        }
    }

    async fn dispatch_message(&self, message: MessageToBackend) {
        match message {
            MessageToBackend::ConfigurationRequest => {
                services::config_service::handle_config_request(self).await;
            }
            MessageToBackend::DownloadModelRequest(model) => {
                services::model_service::handle_download_model_request(self, model).await;
            }
        }
    }

    pub async fn send(&self, message: MessageFromBackend) {
        self.tx
            .send(message)
            .await
            .expect("failed to send message to frontend");
    }

    pub async fn send_notification(
        &self,
        _type: subwin_bridge::notification::NotificationType,
        content: impl Into<String>,
    ) {
        self.send(MessageFromBackend::NotificationMessage(
            subwin_bridge::notification::NotificationMessage {
                notification_type: _type,
                message: content.into(),
            },
        ))
        .await;
    }
}

async fn setup_backend(rx: Receiver<MessageToBackend>, tx: Sender<MessageFromBackend>) {
    let (config, cache_path) = crate::config::load_config()
        .await
        .expect("failed to load config");

    let request_client = reqwest::Client::new();
    let state = Arc::new(RwLock::new(State {
        config,
        cache_path,
        request_client,
    }));

    let context = AppContext { state, tx };
    context.consume_bridge_messages(rx).await;
}

pub fn run(rx: Receiver<MessageToBackend>, tx: Sender<MessageFromBackend>) {
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        runtime.block_on(async { setup_backend(rx, tx).await });
    });
}
