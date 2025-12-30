//! Backend runtime setup and orchestration.
//!
//! This module wires together configuration, shared state, and the message
//! dispatch loop that listens to frontend bridge requests.

use std::{sync::Arc, thread};

use subwin_bridge::{MessageFromBackend, MessageToBackend};
use tokio::sync::{
    RwLock,
    mpsc::{Receiver, Sender},
};

use crate::app::AppContext;
use crate::state::State;

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
        active_stream: None,
    }));

    let context = Arc::new(AppContext { state, tx });
    context.consume_bridge_messages(rx).await;
}

/// Spawn the backend runtime and begin processing bridge messages.
pub fn run(rx: Receiver<MessageToBackend>, tx: Sender<MessageFromBackend>) {
    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        runtime.block_on(async { setup_backend(rx, tx).await });
    });
}
