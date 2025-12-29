#[derive(Debug, Clone, Copy, Default)]
pub struct DownloadProgressEvent {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed: f64,
    pub remaining_time: f64,
}

#[derive(Debug, Clone)]
pub struct DownloadEntity {
    pub progress: DownloadProgressEvent,
}

impl DownloadEntity {
    pub fn new(_: &mut gpui::Context<Self>) -> Self {
        Self {
            progress: DownloadProgressEvent::default(),
        }
    }
}

impl gpui::EventEmitter<DownloadProgressEvent> for DownloadEntity {}
impl gpui::Global for DownloadEntity {}
