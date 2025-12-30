use std::{collections::VecDeque, time::Instant};

use whisper_rs::{
    FullParams, WhisperContext, WhisperContextParameters, WhisperError, WhisperState,
};

use crate::{CaptionSegment, Transcriber, milliseconds_to_samples};

/// Real-time Whisper-based audio transcriber.
///
/// This struct buffers incoming mono audio samples and periodically runs
/// overlapping Whisper inference to produce incremental transcription output.
pub struct WhisperTranscriber {
    /// Number of samples received since the last decode attempt.
    since_last_decode: usize,
    /// Temporary buffer used when padding is required.
    scratch_buffer: Vec<f32>,
    /// Rolling window of recent audio samples.
    segment_window: VecDeque<f32>,
    /// Internal Whisper inference state.
    whisper_state: WhisperState,
    /// Target rolling window length, in samples.
    length_samples: usize,
    /// Decode scheduling interval, in samples.
    repeat_run_samples: usize,
    /// Minimum number of samples required for a decode attempt.
    min_transcode_samples: usize,
    target_rate: u32,
    total_samples_seen: i64,
}

impl WhisperTranscriber {
    pub fn new(
        target_rate: u32,
        path: &str,
        context_params: WhisperContextParameters,
    ) -> Result<Self, WhisperError> {
        let min_transcode_samples = WhisperTranscriber::min_transcription_samples(target_rate);
        let length_samples =
            milliseconds_to_samples(crate::CONTEXT_LENGTH_MILLISECONDS, target_rate);
        let repeat_run_samples =
            milliseconds_to_samples(crate::REPEAT_RUN_MILLISECONDS, target_rate);

        let transcoder_context = WhisperContext::new_with_params(path, context_params)?;
        let whisper_state = transcoder_context.create_state()?;
        whisper_rs::install_logging_hooks();

        Ok(Self {
            total_samples_seen: 0,
            target_rate,
            since_last_decode: 0,
            segment_window: VecDeque::with_capacity(length_samples),
            scratch_buffer: Vec::with_capacity(min_transcode_samples),
            whisper_state,
            length_samples,
            repeat_run_samples,
            min_transcode_samples,
        })
    }

    pub fn build_context_params() -> WhisperContextParameters<'static> {
        let mut context_params = WhisperContextParameters::default();
        context_params.use_gpu(true);
        context_params
    }

    pub fn build_request_params() -> FullParams<'static, 'static> {
        let mut params = FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
        // disable some not usable shit
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);
        params.set_debug_mode(false);

        // model and runtime optimizations
        // TODO: re-enable this: params.set_n_threads(num_cpus::get_physical() as i32);
        params.set_no_timestamps(false);
        params.set_token_timestamps(false);
        params.set_single_segment(false);
        // params.set_max_tokens(96);
        params.set_language(None); // TODO: request from end-calling user

        params
    }
}

impl Transcriber<FullParams<'static, 'static>> for WhisperTranscriber {
    fn min_transcription_samples(sample_rate: u32) -> usize {
        (sample_rate as usize) / 10 // expect minimum a second of submitted audio
    }

    fn accept_samples(&mut self, samples: &[f32]) {
        self.segment_window.extend(samples.iter().copied());
        self.since_last_decode += samples.len();

        if self.segment_window.len() > self.length_samples {
            let drop = self.segment_window.len() - self.length_samples;
            self.segment_window.drain(..drop);
        }

        self.total_samples_seen += samples.len() as i64;
    }

    fn try_transcribe(
        &mut self,
        mut params: FullParams<'static, 'static>,
    ) -> (Vec<CaptionSegment>, u128) {
        // fail fast, if there's not enough data to process yet
        if self.since_last_decode < self.repeat_run_samples {
            return (Vec::new(), 0);
        }

        let start = Instant::now();

        // get transcode audio, if there's more enough data for transcode.
        // otherwise, pad with zero-value for the provided type
        let transcode_audio: &[f32] = if self.segment_window.len() >= self.min_transcode_samples {
            self.segment_window.make_contiguous()
        } else {
            self.scratch_buffer.clear();
            self.scratch_buffer
                .extend(self.segment_window.iter().copied());
            self.scratch_buffer.resize(self.min_transcode_samples, 0.0);
            &self.scratch_buffer
        };

        // TODO: make the threshold configurable.
        let rms = super::calculate_samples_rms(transcode_audio);
        if rms == 0.0 || (20.0 * rms.log10()) <= -60.0 {
            self.since_last_decode = 0;
            return (Vec::new(), 0);
        }

        // reset the current model offset and remove unwanted junk
        params.set_offset_ms(0);
        params.set_suppress_nst(true);

        let sample_rate = self.target_rate as i64;
        let window_samples = transcode_audio.len() as i64;
        let window_start_ms = (self.total_samples_seen - window_samples) * 1000 / sample_rate;

        if let Err(e) = self.whisper_state.full(params, transcode_audio) {
            eprintln!("Failed to transcode audio: {e}");
            return (Vec::new(), 0);
        }

        let mut segments = Vec::new();
        for segment in self.whisper_state.as_iter() {
            let text = segment.to_str_lossy().unwrap_or_default();
            if text.trim().is_empty() {
                continue;
            }

            let start_milliseconds = window_start_ms + (segment.start_timestamp() * 10);
            let end_milliseconds = window_start_ms + (segment.end_timestamp() * 10);
            segments.push(CaptionSegment {
                start_milliseconds,
                end_milliseconds,
                text: text.to_string(),
            });
        }

        let duration = start.elapsed().as_millis();
        self.since_last_decode = 0;

        (segments, duration)
    }
}
