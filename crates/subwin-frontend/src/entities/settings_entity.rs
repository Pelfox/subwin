use gpui::{AppContext, Entity};
use subwin_bridge::config::Config;

#[derive(Debug, Clone, Default)]
pub struct SettingsEntity {
    pub config: Config,
}

impl SettingsEntity {
    pub fn update<C: AppContext>(entity: &Entity<Self>, config: Config, cx: &mut C) {
        entity.update(cx, |this, cx| {
            this.config = config;
            cx.notify();
        });
    }
}
