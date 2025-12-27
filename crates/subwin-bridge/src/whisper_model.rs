/// Available Whisper transcription models for download and local inference.
#[derive(Debug, Clone)]
pub enum WhisperModel {
    // Tiny models.
    TinyQuantized8,
    TinyQuantized5,
    Tiny,
    // Small models.
    SmallQuantized8,
    SmallQuantized5,
    Small,
    // Base models.
    BaseQuantized8,
    BaseQuantized5,
    Base,
    // Medium models.
    MediumQuantized8,
    MediumQuantized5,
    Medium,
    // Large models.
    LargeTurboQuantized8,
    LargeTurboQuantized5,
    LargeTurbo,
    LargeQuantized5,
    Large,
}
