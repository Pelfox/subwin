use gpui::Entity;

pub mod audio_devices_entity;
pub mod download_entity;
pub mod settings_entity;

#[derive(Debug, Clone, Default)]
pub struct CaptionsEntity {
    pub last_run_duration: u128,
    pub last_run_content: String,
}

#[derive(Debug, Clone)]
pub struct DataEntities {
    pub settings: Entity<settings_entity::SettingsEntity>,
    pub download: Entity<download_entity::DownloadEntity>,
    pub audio_devices: Entity<audio_devices_entity::AudioDevicesEntity>,
    pub captions: Entity<CaptionsEntity>,
}
