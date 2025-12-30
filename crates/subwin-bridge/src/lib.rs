//! Communication bridge between frontend and backend.
//!
//! This crate defines the types and protocols used to connect a graphical
//! frontend with an asynchronous backend responsible for audio capture,
//! Whisper transcription, model management, and more.
//!
//! The design is deliberately lightweight and unidirectional:
//! - The frontend sends commands (e.g., start transcription, download
//!   model, request config).
//! - The backend pushes events (e.g., new subtitle segments,
//!   notifications, download progress).
//!
//! Communication happens over bounded [`tokio::sync::mpsc`] channels wrapped
//! in [`BridgeChannels`], providing back-pressure, async compatibility, and
//! clean separation of concerns.

pub mod audio;
pub mod config;
pub mod notification;
pub mod whisper_model;

use tokio::sync::mpsc::{self, Receiver, Sender};

/// Messages emitted by the backend to inform the frontend of state updates.
///
/// These are typically sent in response to frontend requests or to push
/// asynchronous progress/events (e.g., download progress, notifications).
#[derive(Debug, Clone)]
pub enum MessageFromBackend {
    /// Generic message for all notifications in the application.
    NotificationMessage(notification::NotificationMessage),
    /// Response to the configuration request from the frontend.
    ConfigurationResponse(config::Config),
    /// Generic message for reporting the progress of a download.
    DownloadProgressUpdate {
        /// Current speed in bytes per second.
        speed: f64,
        /// Amount of downloaded bytes to this point.
        downloaded_bytes: u64,
        /// Overall amount of bytes to be downloaded.
        total_bytes: u64,
        /// Estimated remaining time until download completion, in seconds.
        remaining_time: f64,
    },
    AudioDevicesListResponse(Vec<audio::InputDevice>),
    TranscriptionStartedResponse,
    TranscriptionStateUpdate {
        time_taken: u128,
        new_segment_text: String,
    },
}

/// Commands issued by the frontend to control or query the backend.
///
/// These messages drive the core functionality of the application.
#[derive(Debug, Clone)]
pub enum MessageToBackend {
    /// Request for the application configuration.
    ConfigurationRequest,
    /// Request to start downloading a model.
    DownloadModelRequest(whisper_model::WhisperModel),
    AudioDevicesListRequest,
    SelectAudioDevice(String),
    StartTranscriptionRequest,
}

/// Paired `tokio::mpsc` channels for bidirectional communication between
/// frontend and backend.
pub struct BridgeChannels {
    /// Receiver used by the frontend to get messages from the backend.
    pub frontend_rx: Receiver<MessageFromBackend>,
    /// Sender used by the frontend to send commands to the backend.
    pub frontend_tx: Sender<MessageToBackend>,

    /// Receiver used by the backend to get commands from the frontend.
    pub backend_rx: Receiver<MessageToBackend>,
    /// Sender used by the backend to send events/responses to the frontend.
    pub backend_tx: Sender<MessageFromBackend>,
}

impl BridgeChannels {
    /// Creates a new pair of bridged channels with the given buffer capacity.
    pub fn new(buffer: usize) -> Self {
        let (to_backend_tx, to_backend_rx) = mpsc::channel(buffer);
        let (to_frontend_tx, to_frontend_rx) = mpsc::channel(buffer);
        Self {
            frontend_tx: to_backend_tx,
            frontend_rx: to_frontend_rx,
            backend_rx: to_backend_rx,
            backend_tx: to_frontend_tx,
        }
    }
}

impl Default for BridgeChannels {
    fn default() -> Self {
        Self::new(64)
    }
}
