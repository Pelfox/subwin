use std::collections::VecDeque;

use whisper_rs::{
    FullParams, WhisperContext, WhisperContextParameters, WhisperError, WhisperState,
};

use crate::{Transcriber, milliseconds_to_samples};

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
    /// End timestamp (centiseconds) of the last emitted segment.
    last_end_cs: i64,
    /// Target rolling window length, in samples.
    length_samples: usize,
    /// Decode scheduling interval, in samples.
    repeat_run_samples: usize,
    /// Minimum number of samples required for a decode attempt.
    min_transcode_samples: usize,
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

        Ok(Self {
            since_last_decode: 0,
            segment_window: VecDeque::with_capacity(length_samples),
            scratch_buffer: Vec::with_capacity(min_transcode_samples),
            whisper_state,
            last_end_cs: 0,
            length_samples,
            repeat_run_samples,
            min_transcode_samples,
        })
    }
}

impl Transcriber<FullParams<'static, 'static>> for WhisperTranscriber {
    fn min_transcription_samples(sample_rate: u32) -> usize {
        (sample_rate as usize * 12) / 10 // ~1.2s
    }

    fn accept_samples(&mut self, samples: &[f32]) {
        self.segment_window.extend(samples.iter().copied());
        self.since_last_decode += samples.len();

        if self.segment_window.len() > self.length_samples {
            let drop = self.segment_window.len() - self.length_samples;
            self.segment_window.drain(..drop);
        }
    }

    fn try_transcribe(&mut self, params: FullParams<'static, 'static>) -> Option<String> {
        // fail fast, if there's not enough data to process yet
        if self.since_last_decode < self.repeat_run_samples {
            return None;
        }

        // TODO: check if last ~100-150ms are silent via RMS

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

        if let Err(e) = self.whisper_state.full(params, transcode_audio) {
            eprintln!("Failed to transcode audio: {e}");
            return None;
        }

        self.since_last_decode = 0;
        let mut segment_string = String::new();
        let mut new_last_end_cs = self.last_end_cs;

        for segment in self.whisper_state.as_iter() {
            let seg_end_cs = segment.end_timestamp();
            if seg_end_cs <= self.last_end_cs {
                continue; // a new block is already emitted
            }

            if let Ok(text) = segment.to_str()
                && !text.trim().is_empty()
            {
                segment_string.push_str(text);
                segment_string.push(' ');
            }
            new_last_end_cs = seg_end_cs;
        }

        self.last_end_cs = new_last_end_cs;
        let output = segment_string.trim().to_string();

        (!output.is_empty()).then_some(output)
    }
}
