use std::time::Instant;

use log::{error, info, warn};
use whisper_rs::{
    FullParams, WhisperContext, WhisperContextParameters, WhisperError, WhisperState,
};

use crate::speech::{
    CONTEXT_LENGTH_MILLISECONDS, REPEAT_RUN_MILLISECONDS, Transcoder, milliseconds_to_samples,
};

pub struct WhisperTranscoder {
    segment_window: Vec<f32>,
    scratch_buffer: Vec<f32>,
    since_last_decode: usize, // samples
    transcoder_state: WhisperState,

    length_samples: usize,
    repeat_run_samples: usize,
    min_transcode_samples: usize,
}

impl WhisperTranscoder {
    pub fn new(
        target_rate: u32,
        path: &str,
        context_params: WhisperContextParameters,
    ) -> Result<Self, WhisperError> {
        let min_transcode_samples = WhisperTranscoder::min_transcode_samples(target_rate);
        let length_samples = milliseconds_to_samples(CONTEXT_LENGTH_MILLISECONDS, target_rate);
        let repeat_run_samples = milliseconds_to_samples(REPEAT_RUN_MILLISECONDS, target_rate);

        let transcoder_context = WhisperContext::new_with_params(path, context_params)?;
        let transcoder_state = transcoder_context.create_state()?;

        Ok(Self {
            segment_window: Vec::with_capacity(length_samples),
            scratch_buffer: Vec::with_capacity(min_transcode_samples),
            since_last_decode: 0,
            transcoder_state,

            length_samples,
            repeat_run_samples,
            min_transcode_samples,
        })
    }
}

impl Transcoder<FullParams<'static, 'static>> for WhisperTranscoder {
    fn min_transcode_samples(sample_rate: u32) -> usize {
        (sample_rate as f32 * 0.80) as usize // around 80% of a second
    }

    #[inline]
    fn accept_samples(&mut self, samples: &[f32]) {
        self.segment_window.extend_from_slice(samples);
        self.since_last_decode += samples.len();

        // if transcoder can't keep up, keep only the last `length_samples`
        if self.segment_window.len() > self.length_samples {
            let samples_to_drop = self.segment_window.len() - self.length_samples;
            self.segment_window.drain(..samples_to_drop);
            warn!("Can't keep up! {samples_to_drop} samples behind.");
        }
    }

    #[inline]
    fn try_transcode(&mut self, params: FullParams<'static, 'static>) -> Option<String> {
        // fail fast, if there's not enough data to process yet
        if self.since_last_decode < self.repeat_run_samples {
            return None;
        }

        // TODO: check if last ~100-150ms are silent via RMS

        // get transcode audio, if there's more enough data for transcode.
        // otherwise, pad with zero-value for the provided type
        let transcode_audio: &[f32] = if self.segment_window.len() >= self.min_transcode_samples {
            &self.segment_window
        } else {
            self.scratch_buffer.clear();
            self.scratch_buffer.extend_from_slice(&self.segment_window);
            self.scratch_buffer.resize(self.min_transcode_samples, 0.0);
            &self.scratch_buffer
        };

        let start_time = Instant::now();
        if let Err(e) = self.transcoder_state.full(params, transcode_audio) {
            error!("Failed to transcode audio: {e}");
            return None;
        }

        let mut output = String::new();
        for segment in self.transcoder_state.as_iter() {
            output.push_str(segment.to_str().expect("failed to get segment's content"));
        }

        let elapsed_since_start = start_time.elapsed().as_millis();
        info!("Transcription took {elapsed_since_start:.0}ms.");

        Some(output)
    }
}
