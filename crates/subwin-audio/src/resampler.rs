use rubato::{FftFixedInOut, Resampler, ResamplerConstructionError};

/// Errors that can occur during audio resampling.
///
/// This error type represents failures caused by invalid input/output provided
/// to the resampler.
#[derive(Debug, thiserror::Error)]
pub enum ResamplerError {
    /// The input buffer length does not match the resampler’s required block
    /// size.
    ///
    /// This error is returned when the number of input samples provided by the
    /// caller differs from the number of samples expected by the resampler for
    /// the current processing step.
    #[error("invalid input length for resampler: expected {expected} samples, got {actual}")]
    InvalidInputLength {
        /// Number of input samples expected by the resampler.
        expected: usize,
        /// Number of input samples provided by the caller.
        actual: usize,
    },
    /// Failed to resample the provided audio samples.
    ///
    /// This error is returned when the underlying resampling engine encounters
    /// a failure while processing input samples.
    #[error("failed to resample input samples: {0}")]
    ResampleError(#[from] rubato::ResampleError),
}

/// Real-time audio stream resampler trait.
///
/// This trait defines a common interface for resampling a continuous audio
/// stream in real time. Implementations consume input samples and deliver
/// resampled output through a user-provided callback.
///
/// All implementations are expected to be suitable for real-time audio
/// processing:
/// - They must not allocate memory during processing.
/// - They must be efficient for small and medium buffer sizes (typically
///   15–4096 frames at 48 kHz).
/// - They must be thread-safe.
pub trait AudioResampler<T: rubato::Sample>: Send {
    /// Process an input audio buffer and emit resampled output via a callback.
    ///
    /// The input slice contains mono audio samples (single channel,
    /// non-interleaved). Implementations may consume all or only part of the
    /// input immediately, depending on their internal buffering strategy.
    ///
    /// The provided callback is invoked zero or more times with contiguous
    /// slices of resampled output data.
    ///
    /// # Returns
    /// Returns the total number of output samples written during this call.
    ///
    /// # Errors
    /// Returns [`ResamplerError`] if resampling fails or if the input does not
    /// meet implementation-specific requirements.
    fn process_callback(
        &mut self,
        input: &[T],
        callback: &mut dyn FnMut(&[T]),
    ) -> Result<usize, ResamplerError>;
}

/// Fixed-block-size FFT-based resampler.
///
/// This resampler implements high-quality, fixed-ratio resampling using
/// FFT-based convolution. It operates strictly on fixed-size input blocks and
/// produces a fixed number of output samples per processing call.
///
/// This type is best suited for systems where audio buffers are already
/// delivered in consistent, known sizes.
pub struct FixedBlockResampler<T: rubato::Sample> {
    input_buffer: Vec<T>,
    output_buffer: Vec<T>,
    resampler: FftFixedInOut<T>,
}

impl<T: rubato::Sample> FixedBlockResampler<T> {
    /// Creates a new fixed-block-size FFT-based resampler for mono audio.
    ///
    /// The resampler expects exactly `block_size` input samples on every call
    /// to [`AudioResampler::process_callback`] and will return an error if the
    /// input size differs. The number of output samples produced per call is
    /// constant.
    ///
    /// This function performs internal memory allocations and should be called
    /// during initialization, not from a real-time audio thread.
    ///
    /// # Errors
    /// Returns [`ResamplerConstructionError`] if the resampler cannot be
    /// constructed with the given parameters.
    pub fn new(
        original_rate: u32,
        target_rate: u32,
        block_size: u32,
    ) -> Result<Self, ResamplerConstructionError> {
        let resampler = FftFixedInOut::new(
            original_rate as usize,
            target_rate as usize,
            block_size as usize,
            1, // we're using mono
        )?;

        let raw_input_buffer = resampler.input_buffer_allocate(true);
        let raw_output_buffer = resampler.output_buffer_allocate(true);

        Ok(Self {
            input_buffer: raw_input_buffer[0].clone(),
            output_buffer: raw_output_buffer[0].clone(),
            resampler,
        })
    }
}

impl<T: rubato::Sample> AudioResampler<T> for FixedBlockResampler<T> {
    fn process_callback(
        &mut self,
        input: &[T],
        callback: &mut dyn FnMut(&[T]),
    ) -> Result<usize, ResamplerError> {
        let expected_len = self.resampler.input_frames_next();
        if input.len() != expected_len {
            return Err(ResamplerError::InvalidInputLength {
                expected: expected_len,
                actual: input.len(),
            });
        }

        if self.input_buffer.len() != expected_len {
            self.input_buffer.resize(expected_len, T::zero());
        }
        self.input_buffer.copy_from_slice(input);

        let input_buffer = &[&self.input_buffer];
        let output_buffer = &mut [&mut self.output_buffer];
        let (_, output_written) =
            self.resampler
                .process_into_buffer(input_buffer, output_buffer, None)?;

        // don't call callback if nothing was written
        if output_written > 0 {
            callback(&self.output_buffer[..output_written]);
        }
        Ok(output_written)
    }
}

/// FFT-based streaming resampler for arbitrary input and output block sizes.
///
/// This resampler is designed for real-time streaming scenarios where input
/// buffers may arrive in unpredictable sizes, including partial audio frames.
///
/// It internally buffers incoming samples in a FIFO queue and feeds the
/// resampling engine whenever enough data is available. Output samples are
/// produced as soon as possible and delivered via the callback.
pub struct StreamingResampler<T: rubato::Sample> {
    resampler: FftFixedInOut<T>,
    frames_queue: std::collections::VecDeque<T>,

    input_buffer: Vec<T>,
    output_buffer: Vec<T>,
}

impl<T: rubato::Sample> StreamingResampler<T> {
    /// Creates a new FFT-based streaming resampler for mono audio.
    ///
    /// Unlike [`FixedBlockResampler`], this resampler does not require a fixed
    /// number of input samples per processing call. Any number of input
    /// samples may be provided, including zero or partial frames.
    ///
    /// The `block_size` parameter controls the internal FFT processing size
    /// and therefore affects latency and performance, but it does not impose
    /// any constraints on the public API.
    ///
    /// This function performs internal memory allocations and should be called
    /// during initialization, not from a real-time audio thread.
    ///
    /// # Errors
    /// Returns [`ResamplerConstructionError`] if the resampler cannot be
    /// constructed with the given parameters.
    pub fn new(
        original_rate: u32,
        target_rate: u32,
        block_size: u32,
    ) -> Result<Self, ResamplerConstructionError> {
        let resampler = FftFixedInOut::new(
            original_rate as usize,
            target_rate as usize,
            block_size as usize,
            1, // we're using mono
        )?;

        let raw_input_buffer = resampler.input_buffer_allocate(true);
        let raw_output_buffer = resampler.output_buffer_allocate(true);

        Ok(Self {
            // FIXME: in bursts, can allocate. pre-allocate via VecDeque::extend
            frames_queue: std::collections::VecDeque::new(),
            input_buffer: raw_input_buffer[0].clone(),
            output_buffer: raw_output_buffer[0].clone(),
            resampler,
        })
    }
}

impl<T: rubato::Sample> AudioResampler<T> for StreamingResampler<T> {
    fn process_callback(
        &mut self,
        input: &[T],
        callback: &mut dyn FnMut(&[T]),
    ) -> Result<usize, ResamplerError> {
        let mut total_written = 0usize;
        self.frames_queue.extend(input);

        loop {
            let wanted_len = self.resampler.input_frames_next();
            if self.frames_queue.len() < wanted_len {
                break;
            }

            if self.input_buffer.len() != wanted_len {
                self.input_buffer.resize(wanted_len, T::zero());
            }

            for i in 0..wanted_len {
                let frame_value = self
                    .frames_queue
                    .pop_front()
                    .expect("failed to pop a frame value");
                self.input_buffer[i] = frame_value;
            }

            let input_buffer = &[&self.input_buffer];
            let output_buffer = &mut [&mut self.output_buffer];
            let (_, output_written) =
                self.resampler
                    .process_into_buffer(input_buffer, output_buffer, None)?;

            // don't call callback if nothing was written
            if output_written > 0 {
                callback(&self.output_buffer[..output_written]);
                total_written += output_written;
            }
        }

        Ok(total_written)
    }
}
