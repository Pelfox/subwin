use std::str::FromStr;

use cpal::{
    Device, Host,
    traits::{DeviceTrait, HostTrait},
};

// TODO: add functions to get host by its ID.

/// Errors that can occur while configuring or creating an audio input device.
///
/// This error type represents failures that may occur during input stream
/// setup, including device configuration discovery and stream construction.
#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    /// Failed to enumerate audio input devices. This error occurs when the
    /// underlying audio backend fails to query the list of available input
    /// devices for the host.
    #[error("failed to read device's information: {0}")]
    ReadDevices(#[from] cpal::DevicesError),
    /// Failed to construct an input audio stream. This error is returned when
    /// the audio backend rejects the requested stream configuration or fails
    /// to initialize the input stream.
    #[error("failed to build device input stream: {0}")]
    BuildStream(#[from] cpal::BuildStreamError),
    /// Failed to obtain the device’s default input stream configuration. This
    /// error occurs when the device does not support input streams or when the
    /// audio backend fails to query the default input configuration.
    #[error("failed to build device config: {0}")]
    BuildStreamConfig(#[from] cpal::DefaultStreamConfigError),
    /// Failed to parse the provided device ID. It may be incorrect or invalid.
    /// You should refer to CPAL's error for more information.
    #[error("failed to parse device id: {0}")]
    ReadDeviceId(#[from] cpal::DeviceIdError),
}

/// Represents parsed input audio device belonging to a specific host.
#[derive(Clone)]
pub struct HostInputDevice {
    /// Unique identifier of the device within the host.
    pub id: cpal::DeviceId,
    /// Human-readable device description.
    pub description: String,

    device: Device,
}

impl std::fmt::Display for HostInputDevice {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} ({})", self.description, self.id)
    }
}

impl HostInputDevice {
    /// Returns the default input sample rate and channel count for this device.
    ///
    /// This method queries the device’s default input stream configuration and
    /// returns the sample rate and number of channels selected by the audio
    /// backend as its preferred input settings.
    pub fn sample_rate_and_channels(&self) -> Result<(cpal::SampleRate, u16), DeviceError> {
        let default_input_config = self.device.default_input_config()?;
        Ok((
            default_input_config.sample_rate(),
            default_input_config.channels(),
        ))
    }

    /// Returns a preferred input buffer size adjusted for the target sample rate.
    ///
    /// This method queries the device’s default input stream configuration and
    /// computes a buffer size that is compatible with both the device’s native
    /// sample rate and the requested `target_rate`.
    pub fn target_buffer_size(&self, target_rate: u32) -> Result<u32, DeviceError> {
        let default_input_config = self.device.default_input_config()?;
        let device_buffer_size = match default_input_config.buffer_size() {
            cpal::SupportedBufferSize::Range { max, .. } => *max,
            cpal::SupportedBufferSize::Unknown => super::FIXED_FRAME_COUNT,
        };

        // calculate the target buffer size with accordance to the sample rate
        // ratio, since rubato wants a buffer size that is denominated to the
        // target sample rate
        let original_sample_rate = default_input_config.sample_rate();
        let rate_denominator = crate::gcd(original_sample_rate, target_rate);
        Ok(crate::find_nearest_to(
            device_buffer_size,
            original_sample_rate / rate_denominator,
        ))
    }
}

/// Returns a list of all input audio devices available on the given host.
///
/// This function queries the provided [`cpal::Host`] for all input-capable audio
/// devices and returns their identifiers and display names.
pub fn list_host_input_devices(host: &Host) -> Result<Vec<HostInputDevice>, DeviceError> {
    Ok(host
        .input_devices()?
        .map(|device| HostInputDevice {
            id: device.id().expect("failed to obtain device's id"),
            description: device
                .description()
                .expect("failed to obtain device's information")
                .to_string(),
            device,
        })
        .collect())
}

/// Creates and returns an input audio stream for the given device using its
/// default input configuration.
///
/// This function builds an input stream based on the device’s default input
/// stream configuration and applies an internally derived fixed buffer size.
/// It registers two callbacks:
/// - `callback` is invoked on the audio thread whenever a buffer of input
///   samples becomes available.
/// - `error_callback` is invoked on the audio thread if a runtime stream error
///   occurs.
///
/// # Buffering
///
/// The stream uses a fixed buffer size derived from the device’s reported
/// capabilities via [`HostInputDevice::target_buffer_size`]. This improves
/// determinism and is suitable for real-time audio processing.
///
/// # Threading
///
/// Both `callback` and `error_callback` are executed on a real-time audio thread.
/// They must:
/// - Be fast and non-blocking.
/// - Avoid memory allocation.
/// - Avoid locks and I/O.
///
/// Blocking operations in callbacks may cause audio dropouts or undefined
/// behavior.
pub fn open_cpal_input_stream<T>(
    input_device: &HostInputDevice,
    target_rate: u32,
    mut callback: impl FnMut(&[T]) + Send + 'static,
    error_callback: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, DeviceError>
where
    T: cpal::SizedSample + cpal::Sample,
{
    let mut default_input_config: cpal::StreamConfig =
        input_device.device.default_input_config()?.into();
    default_input_config.buffer_size =
        cpal::BufferSize::Fixed(input_device.target_buffer_size(target_rate)?);

    Ok(input_device.device.build_input_stream(
        &default_input_config,
        move |data: &[T], _| callback(data),
        error_callback,
        None,
    )?)
}

/// Retrieves a specific audio device by its unique identifier within a given
/// host.
///
/// Attempts to look up an input or output device using a string
/// representation of its [`cpal::DeviceId`].
pub fn get_device_by_id(host: &Host, device_id: String) -> Result<Option<Device>, DeviceError> {
    let device_id = cpal::DeviceId::from_str(&device_id)?;
    Ok(host.device_by_id(&device_id))
}
