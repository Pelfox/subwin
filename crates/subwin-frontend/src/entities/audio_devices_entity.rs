#[derive(Debug, Clone, Default)]
pub struct AudioDevicesEntity {
    pub audio_devices: Vec<subwin_bridge::audio::InputDevice>,
}
