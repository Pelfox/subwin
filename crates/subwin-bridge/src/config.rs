use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Controls the appearance of the background behind captions window.
/// This enum determines how the area behind the caption text is rendered.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptionsBackgroundAppearance {
    /// Solid, fully opaque background behind the caption text.
    #[serde(alias = "opaque")]
    Opaque,
    /// No background; text is drawn directly over the underlying contents.
    Transparent,
    /// Semi-transparent blurred background. Not always supported. Default value.
    #[default]
    Blurred,
}

/// Configuration for the display and styling of captions. This
/// struct controls key visual aspects of how captions are rendered on screen.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CaptionsConfig {
    /// Distance in pixels from the bottom of the screen to the caption box in
    /// pixels.
    pub padding_from_bottom: u32,
    /// Visual style of the background behind the caption text.
    pub background_appearance: CaptionsBackgroundAppearance,
    /// Width of the caption text window in pixels.
    pub window_width: f32,
    /// Height of the caption text window in pixels.
    pub window_height: f32,
}

impl Default for CaptionsConfig {
    fn default() -> Self {
        Self {
            padding_from_bottom: 180,
            background_appearance: CaptionsBackgroundAppearance::default(),
            window_width: 700.0,
            window_height: 80.0,
        }
    }
}

/// Configuration for selecting specific audio devices and backends.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AudioDeviceConfig {
    /// Identifier of the preferred audio host/backend.
    pub selected_host_id: Option<String>,
    /// Identifier of the preferred audio input device.
    pub selected_device_id: Option<String>,
}

impl Default for AudioDeviceConfig {
    fn default() -> Self {
        Self {
            selected_host_id: None,
            selected_device_id: None,
        }
    }
}

/// Global application configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Whether to enable transcoding pipeline for input audio.
    pub enable_transcoder: bool,
    /// Whether to insert automatic translation into a transcoding pipeline.
    pub enable_auto_translation: bool,
    /// Configuration for the captions module of the application.
    pub captions_config: CaptionsConfig,
    /// Path to the active transcription model, if any.
    pub active_model_path: Option<PathBuf>,
    /// Configuration for audio devices for the host.
    pub audio_device_config: AudioDeviceConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enable_transcoder: true,
            enable_auto_translation: true,
            captions_config: CaptionsConfig::default(),
            active_model_path: None,
            audio_device_config: AudioDeviceConfig::default(),
        }
    }
}
