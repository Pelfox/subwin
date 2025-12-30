//! Real-time transcription scheduling primitives and STT processing
//! implementations.
//!
//! This crate provides foundational timing constants and a trait used to drive
//! continuous, overlapping transcriptions of live audio input. It enables
//! low-latency, incremental captioning by repeatedly processing recent audio
//! context.

pub mod stabilizer;
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

#[derive(Debug, Clone)]
pub struct CaptionSegment {
    pub start_milliseconds: i64,
    pub end_milliseconds: i64,
    pub text: String,
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

    /// Attempts to transcribe the current accumulated audio segment using the
    /// underlying transcription logic.
    ///
    /// This method is called each time a new sample data arrives, so
    /// implementations should check and fail fast, if there aren't enough data
    /// available.
    ///
    /// # Returns
    ///
    /// A tuple containing:
    ///
    /// * `Option<String>` - The concatenated transcribed text from the current
    ///   audio segment, if any text was produced. Returns `None` if no speech
    ///   was detected, no text was generated, or if processing failed.
    /// * `u128` - The elapsed time for the transcription inference in
    ///   milliseconds.
    fn try_transcribe(&mut self, params: P) -> (Vec<CaptionSegment>, u128);
}

pub(crate) fn calculate_samples_rms<T>(samples_data: &[T]) -> f64
where
    T: Copy + std::ops::Mul<Output = T> + Into<f64>,
{
    if samples_data.is_empty() {
        return 0.0;
    }

    let length = samples_data.len() as f64;
    let sum_of_squares: f64 = samples_data
        .iter()
        .copied()
        .map(Into::into)
        .map(|value: f64| value * value)
        .sum();

    (sum_of_squares / length).sqrt()
}
