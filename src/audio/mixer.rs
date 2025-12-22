/// Mixes interleaved stereo audio samples down to mono.
///
/// This function converts a stereo audio buffer into mono by averaging the
/// left and right channels for each frame: `mono = (left + right) * 0.5`.
///
/// The input slice must contain interleaved stereo samples in the form
/// `[L0, R0, L1, R1, ...]`. The resulting mono samples are written into
/// `samples_accumulator`.
///
/// # Returns
/// Returns the number of mono frames written to `samples_accumulator`.
pub fn mix_stereo_to_mono<T>(samples_accumulator: &mut [T], samples_frame_data: &[T]) -> usize
where
    T: Copy
        + num_traits::identities::Zero
        + num_traits::FromPrimitive
        + std::ops::Add<Output = T>
        + std::ops::Mul<Output = T>,
{
    let frames = samples_frame_data.len() / 2;
    let half = T::from_f32(0.5).expect("failed to obtain a half");
    for i in 0..frames {
        let left_channel_sample = samples_frame_data[i * 2];
        let right_channel_sample = samples_frame_data[(i * 2) + 1];
        samples_accumulator[i] = (left_channel_sample + right_channel_sample) * half;
    }
    frames
}
