//! Audio utilities for capturing, mixing, and resampling input streams.
//!
//! This crate wraps low-level audio building blocks into a small set of
//! helpers that are oriented toward real-time input capture and mono
//! processing. It focuses on:
//! - Enumerating input devices and building input streams with `cpal`.
//! - Converting interleaved stereo frames to mono samples.
//! - Resampling mono audio streams with FFT-based resamplers.
//!
//! # Real-time constraints
//! Audio callbacks run on a real-time thread. Avoid allocations, locks, and
//! blocking I/O inside callbacks whenever possible.

pub mod device;
pub mod mixer;
pub mod resampler;

/// A fallback fixed buffer size (in frames) used when the audio device reports
/// an unknown supported buffer size.
///
/// It is used in `target_buffer_size` when `cpal` cannot determine the
/// device's preferred or maximum buffer size.
pub const FIXED_FRAME_COUNT: u32 = 4096;

/// Computes the greatest common divisor (GCD) of two unsigned integers.
///
/// This function implements the classic Euclidean algorithm.
pub(crate) fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let temp = a % b;
        a = b;
        b = temp;
    }
    a
}

/// Rounds `base` to the nearest multiple of `denominator`.
///
/// This function finds the closest integer to `base` that is evenly divisible
/// by `denominator`. In case of a tie (exactly halfway between two multiples),
/// it rounds away from zero (i.e., upward when `remainder * 2 == denominator`).
pub(crate) fn find_nearest_to(base: u32, denominator: u32) -> u32 {
    let remainder = base % denominator;
    if remainder * 2 <= denominator {
        base - remainder
    } else {
        base - remainder + denominator
    }
}
