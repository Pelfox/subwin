use std::path::PathBuf;

use cpal::traits::StreamTrait;
use ringbuf_blocking::{
    BlockingHeapRb,
    traits::{Consumer, Producer, Split},
};
use subwin_audio::{
    device::HostInputDevice,
    resampler::{AudioResampler, StreamingResampler},
};
use subwin_bridge::notification::NotificationType;
use subwin_speech::{
    CaptionSegment, Transcriber, stabilizer::CaptionsStabilizer, whisper::WhisperTranscriber,
};

/// Target sample rate for the transcription pipeline.
const TARGET_RATE: u32 = 16_000;

/// History window length for the captions stabilizer, in milliseconds.
const STABILIZER_WINDOW_MILLISECONDS: i64 = 1500;

/// Aggregates inputs required to build a transcription session.
struct TranscriptionInputs {
    /// Path to the active Whisper model on disk.
    active_model_path: PathBuf,
    /// Selected audio device converted to a host-aware wrapper.
    active_device: HostInputDevice,
}

/// Represents derived settings for the active audio device.
struct AudioDeviceSettings {
    /// The device's default input sample rate.
    sample_rate: cpal::SampleRate,
    /// The number of input channels reported by the device.
    channels: u16,
    /// The target buffer size for capture, expressed in frames.
    target_buffer_size: u32,
}

/// Holds mutable state for the audio callback (resampling and mixing).
struct ResampleCallbackState {
    /// Number of audio channels in the incoming stream
    channels: u16,
    /// Target chunk size (in mono samples) before forwarding to the transcoder.
    target_buffer_size: u32,
    /// Streaming resampler instance handling rate conversion.
    resampler: StreamingResampler<f32>,
    /// Accumulator for a downmixed mono f32 samples across callbacks.
    samples_accumulator: Vec<f32>,
}

impl ResampleCallbackState {
    /// Create a new resampling callback state with pre-allocated buffers.
    fn new(
        sample_rate: cpal::SampleRate,
        target_rate: u32,
        target_buffer_size: u32,
        channels: u16,
    ) -> Self {
        Self {
            channels,
            target_buffer_size,
            resampler: StreamingResampler::<f32>::new(sample_rate, target_rate, target_buffer_size)
                .expect("failed to create a resampler"),
            samples_accumulator: Vec::with_capacity(target_buffer_size as usize),
        }
    }

    /// Convert interleaved input to mono and resample it into the ring buffer.
    fn process_input<P: Producer<Item = f32>>(&mut self, data: &[f32], producer: &mut P) {
        let expected_samples = self.target_buffer_size as usize * self.channels as usize;
        if data.len() != expected_samples {
            log::error!(
                "Received an unexpected buffer from CPAL with the size of {} samples. Should be switching to a StreamingResampler?",
                data.len(),
            );
            return;
        }

        let received_frames = data.len() / self.channels as usize;
        if received_frames > self.samples_accumulator.len() {
            log::warn!(
                "Resizing the accumulator (allocation trigger) on the audio thread! Resizing from {} to {}",
                self.samples_accumulator.len(),
                received_frames,
            );
        }

        self.samples_accumulator.resize(received_frames, 0.0);
        subwin_audio::mixer::mix_stereo_to_mono(
            &mut self.samples_accumulator[..received_frames],
            data,
        );

        // push the resampled data and notify the worker
        let mut resampled_callback = |written_data: &[f32]| {
            producer.push_slice(written_data);
        };

        if let Err(err) = self.resampler.process_callback(
            &self.samples_accumulator[..received_frames],
            &mut resampled_callback,
        ) {
            log::error!(
                "Resampler caught an error: {err:?}, received_frames={received_frames}, target_buffer_size={target_buffer_size}",
                target_buffer_size = self.target_buffer_size,
            );
        }
    }
}

/// Join a list of caption segments into a single string with spaces.
fn segments_to_text(segments: &[CaptionSegment]) -> String {
    let mut parts = Vec::with_capacity(segments.len());
    for segment in segments {
        let text = segment.text.trim();
        if !text.is_empty() {
            parts.push(text);
        }
    }
    parts.join(" ")
}

/// Merge history and active caption segments into the latest display string.
fn compose_caption_text(history: &[CaptionSegment], active: &[CaptionSegment]) -> String {
    let history_text = segments_to_text(history);
    let active_text = segments_to_text(active);

    if history_text.is_empty() {
        active_text
    } else if active_text.is_empty() {
        history_text
    } else {
        format!("{history_text} {active_text}")
    }
}

/// Validate config/device state and resolve the inputs needed for transcription.
async fn load_transcription_inputs(
    context: &super::AppContextHandle,
) -> Option<TranscriptionInputs> {
    let (config, active_device) = {
        let state = context.state.read().await;
        (state.config.clone(), state.active_audio_device.clone())
    };

    let active_model_path = match config.active_model_path {
        Some(path) => path,
        None => {
            context
                .send_notification(
                    NotificationType::Error,
                    "Сначала скачайте модель для распознания речи.",
                )
                .await;
            return None;
        }
    };

    if !active_model_path.exists() {
        context
            .send_notification(
                NotificationType::Error,
                "Скачанная модель распознавания речи повреждена.",
            )
            .await;
        // TODO: update config to remove the path
        return None;
    }

    let active_device = match active_device.as_ref() {
        Some(device) => HostInputDevice::from(device.clone()),
        None => {
            context
                .send_notification(
                    NotificationType::Error,
                    "Выберите вводное устройство для захвата звука.",
                )
                .await;
            return None;
        }
    };

    Some(TranscriptionInputs {
        active_model_path,
        active_device,
    })
}

/// Read the device's preferred sample rate and buffer size settings.
fn derive_audio_device_settings(active_device: &HostInputDevice) -> AudioDeviceSettings {
    let (sample_rate, channels) = active_device
        .sample_rate_and_channels()
        .expect("failed to get device's original sample rate and channels");

    let target_buffer_size = active_device
        .target_buffer_size(TARGET_RATE)
        .expect("failed to get target buffer size for the device");

    AudioDeviceSettings {
        sample_rate,
        channels,
        target_buffer_size,
    }
}

/// Spawn a blocking transcription loop that consumes resampled audio frames.
fn spawn_transcription_worker(
    context: super::AppContextHandle,
    target_buffer_size: u32,
    active_model_path: PathBuf,
    mut consumer: impl Consumer<Item = f32> + Send + 'static,
) {
    tokio::task::spawn_blocking(move || {
        let mut transcriber = WhisperTranscriber::new(
            TARGET_RATE,
            active_model_path
                .to_str()
                .expect("failed to decode active transcription model's path"),
            WhisperTranscriber::build_context_params(),
        )
        .expect("failed to create a new Whisper transcriber");

        let params = WhisperTranscriber::build_request_params();
        let mut samples_buffer = vec![0.0f32; target_buffer_size as usize];
        let mut stabilizer = CaptionsStabilizer::new(STABILIZER_WINDOW_MILLISECONDS);

        let mut total_samples_seen: i64 = 0;
        let mut history_segments: Vec<CaptionSegment> = Vec::new();
        let mut active_segments: Vec<CaptionSegment> = Vec::new();
        let mut last_sent_text = String::new();

        loop {
            let len = consumer.pop_slice(&mut samples_buffer);
            if len == 0 {
                continue;
            }

            total_samples_seen += len as i64;
            transcriber.accept_samples(&samples_buffer[..len]);

            let (segments, duration) = transcriber.try_transcribe(params.clone());

            let now_milliseconds = total_samples_seen * 1000 / TARGET_RATE as i64;
            let update = stabilizer.push(now_milliseconds, segments);

            if update.active.is_empty() && update.history.is_empty() {
                continue;
            }

            history_segments.extend(update.history);
            active_segments = update.active;

            let caption_text = compose_caption_text(&history_segments, &active_segments);
            if caption_text.is_empty() || caption_text == last_sent_text {
                continue;
            }

            last_sent_text = caption_text.clone();
            context.send_blocking(
                subwin_bridge::MessageFromBackend::TranscriptionStateUpdate {
                    time_taken: duration,
                    new_segment_text: caption_text,
                },
            );
        }
    });
}

/// Build a CPAL input stream that feeds resampled mono samples into the ring buffer.
fn build_audio_stream(
    active_device: &HostInputDevice,
    device_settings: &AudioDeviceSettings,
    mut producer: impl Producer<Item = f32> + Send + 'static,
) -> cpal::Stream {
    let mut callback_state = ResampleCallbackState::new(
        device_settings.sample_rate,
        TARGET_RATE,
        device_settings.target_buffer_size,
        device_settings.channels,
    );

    subwin_audio::device::open_cpal_input_stream(
        active_device,
        TARGET_RATE,
        move |data: &[f32]| {
            callback_state.process_input(data, &mut producer);
        },
        |error| {
            log::error!("An error occured while processing the input stream data: {error}");
        },
    )
    .expect("failed to open an input stream for the device")
}

/// Handles an incoming transcription start request.
pub async fn handle_start_transcription_request(context: super::AppContextHandle) {
    let inputs = match load_transcription_inputs(&context).await {
        Some(inputs) => inputs,
        None => return,
    };

    let TranscriptionInputs {
        active_model_path,
        active_device,
    } = inputs;

    log::info!("Active device is: {active_device}, active model: {active_model_path:?}");

    let device_settings = derive_audio_device_settings(&active_device);
    log::info!(
        "The target device's original sample rate is {} Hz and it has {} channel(-s). Target buffer size is {}.",
        device_settings.sample_rate,
        device_settings.channels,
        device_settings.target_buffer_size,
    );

    let inner_buffer = BlockingHeapRb::<f32>::new((TARGET_RATE * 3) as usize);
    let (producer, consumer) = inner_buffer.split();

    spawn_transcription_worker(
        context.clone(),
        device_settings.target_buffer_size,
        active_model_path,
        consumer,
    );

    let audio_stream = build_audio_stream(&active_device, &device_settings, producer);
    audio_stream.play().expect("failed to play audio stream");

    {
        let mut state = context.state.write().await;
        state.active_stream = Some(audio_stream);
    }

    log::info!("Started playing the stream...");
    context
        .send(subwin_bridge::MessageFromBackend::TranscriptionStartedResponse)
        .await;
}
