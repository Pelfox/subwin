//! Real-time transcription scheduling primitives and STT processing
//! implementations.
//!
//! This crate provides foundational timing constants and a trait used to drive
//! continuous, overlapping transcriptions of live audio input. It enables
//! low-latency, incremental captioning by repeatedly processing recent audio
//! context.

pub mod whisper;

/// Default context window length in milliseconds.
///
/// This defines how much recent audio (typically 3 seconds) the implementation
/// processes during each transcription attempt. A longer context improves
/// coherence across sentence boundaries and helps with disambiguation, while a
/// shorter one reduces latency and memory usage.
pub const CONTEXT_LENGTH_MILLISECONDS: u32 = 3000;

/// Interval in milliseconds between successive transcription attempts.
///
/// The transcriber is triggered every `REPEAT_RUN_MILLISECONDS` to generate new
/// or refined caption segments. This frequent, overlapping schedule allows the
/// model to incrementally improve previous output and deliver results with
// minimal perceived latency.
pub const REPEAT_RUN_MILLISECONDS: u32 = 500;

/// Converts a duration in milliseconds to the equivalent number of audio samples
/// at the given sample rate.
pub(crate) fn milliseconds_to_samples(milliseconds: u32, sample_rate: u32) -> usize {
    ((sample_rate as u64 * milliseconds as u64) / 1000) as usize
}

/// Trait for real-time audio transcribers that process mono `f32` samples and
/// produce text captions.
///
/// Implementations are expected to:
/// - Buffer incoming audio samples.
/// - Periodically perform inference on the most recent context window
///   (defined by [`CONTEXT_LENGTH_MILLISECONDS`]).
/// - Emit incremental or refined transcription segments.
pub trait Transcriber<P> {
    /// Returns the minimum number of buffered samples required before a
    /// transcription attempt can be performed.
    fn min_transcription_samples(sample_rate: u32) -> usize;

    /// Feeds new mono audio samples (normalized `f32` in range [-1.0, 1.0])
    /// into the transcriber's internal buffer.
    ///
    /// Call this method as frequently as new audio becomes available.
    fn accept_samples(&mut self, samples: &[f32]);

    /// Attempts to perform transcription using the currently buffered audio.
    ///
    /// Returns `Some(String)` containing new or updated caption text if
    /// transcription was performed and produced output, or `None` if no new
    /// text is available.
    fn try_transcribe(&mut self, params: P) -> Option<String>;
}
