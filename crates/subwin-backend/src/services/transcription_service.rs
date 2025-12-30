use cpal::traits::StreamTrait;
use log::info;
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

const TARGET_RATE: u32 = 16_000;

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

pub async fn handle_start_transcription_request(context: super::AppContextHandle) {
    let (config, active_device) = {
        let state = context.state.read().await;
        (state.config.clone(), state.active_audio_device.clone())
    };

    if config.active_model_path.is_none() {
        context
            .send_notification(
                NotificationType::Error,
                "Сначала скачайте модель для распознания речи.",
            )
            .await;
        return;
    }

    let active_model_path = config.active_model_path.unwrap();
    if !active_model_path.exists() {
        context
            .send_notification(
                NotificationType::Error,
                "Скачанная модель распознавания речи повреждена.",
            )
            .await;
        // TODO: update config to remove the path
        return;
    }

    if active_device.is_none() {
        context
            .send_notification(
                NotificationType::Error,
                "Выберите вводное устройство для захвата звука.",
            )
            .await;
        return;
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
            return;
        }
    };

    info!("Active device is: {active_device}, active model: {active_model_path:?}");

    let (sample_rate, channels) = active_device
        .sample_rate_and_channels()
        .expect("failed to get device's original sample rate and channels");

    let target_buffer_size = active_device
        .target_buffer_size(TARGET_RATE)
        .expect("failed to get target buffer size for the device");

    info!(
        "The target device's original sample rate is {} Hz and it has {} channel(-s). Target buffer size is {}.",
        sample_rate, channels, target_buffer_size,
    );

    let mut resampler =
        StreamingResampler::<f32>::new(sample_rate, TARGET_RATE, target_buffer_size)
            .expect("failed to create a resampler");
    let mut samples_accumulator = Vec::with_capacity(target_buffer_size as usize);

    let inner_buffer = BlockingHeapRb::<f32>::new((TARGET_RATE * 3) as usize);
    let (mut producer, mut consumer) = inner_buffer.split();

    let cloned_context = context.clone();
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
        let mut stabilizer = CaptionsStabilizer::new(1500);

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

            log::info!("Got captions update: {update:?}, duration is {duration}ms.");

            history_segments.extend(update.history);
            active_segments = update.active;

            let caption_text = compose_caption_text(&history_segments, &active_segments);
            if caption_text.is_empty() || caption_text == last_sent_text {
                continue;
            }

            last_sent_text = caption_text.clone();
            cloned_context.send_blocking(
                subwin_bridge::MessageFromBackend::TranscriptionStateUpdate {
                    time_taken: duration,
                    new_segment_text: caption_text,
                },
            );
        }
    });

    let mut resampled_callback = move |written_data: &[f32]| {
        producer.push_slice(written_data);
    };

    let audio_stream = subwin_audio::device::open_cpal_input_stream(
        &active_device,
        TARGET_RATE,
        move |data: &[f32]| {
            if data.len() != (target_buffer_size as usize * channels as usize) {
                log::error!("Received an unexpected buffer from CPAL with the size of {} samples. Should be switching to a StreamingResampler?", data.len());
                return;
            }

            let received_frames = data.len() / channels as usize;
            if received_frames > samples_accumulator.len() {
                log::warn!("Resizing the accumulator (allocation trigger) on the audio thread! Resizing from {} to {}", samples_accumulator.len(), received_frames);
            }
            
            samples_accumulator.resize(received_frames, 0.0);
            subwin_audio::mixer::mix_stereo_to_mono(&mut samples_accumulator[..received_frames], data);

            match resampler.process_callback(&samples_accumulator[..received_frames], &mut resampled_callback) {
                Ok(_) => {},
                Err(err) => {
                    log::error!("Resampler caught an error: {err:?}, received_frames={received_frames}, target_buffer_size={target_buffer_size}");
                },
            }
        },
        |error| {
            log::error!("An error occured while processing the input stream data: {error}");
        },
    )
    .expect("failed to open an input stream for the device");

    audio_stream.play().expect("failed to play audio stream");

    {
        let mut state = context.state.write().await;
        state.active_stream = Some(audio_stream);
    }

    info!("Started playing the stream...");
    context
        .send(subwin_bridge::MessageFromBackend::TranscriptionStartedResponse)
        .await;
}
