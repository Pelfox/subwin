mod captions_root_view;
mod overview_page;
mod settings_page;

mod model;

use gpui::{AnyView, AppContext, Context, IntoElement, ParentElement, Render, Styled, Window, div};
use gpui_component::{
    IconName, Root, Side,
    sidebar::{Sidebar, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem},
};

use crate::{
    entities::DataEntities,
    views::{model::ModelPage, overview_page::OverviewPage, settings_page::SettingsPage},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PageUi {
    Overview,
    Settings,
    ModelSettings,
}

pub struct FrontendUi {
    data: DataEntities,
    active_page: PageUi,
    active_page_view: AnyView,
}

impl FrontendUi {
    pub fn new(data: &DataEntities, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let initial_view = cx.new(|cx| OverviewPage::new(data, window, cx)).into();
        Self {
            data: data.clone(),
            active_page: PageUi::Overview,
            active_page_view: initial_view,
        }
    }

    pub fn change_page(&mut self, page: PageUi, window: &mut Window, cx: &mut Context<Self>) {
        let new_page = match page {
            PageUi::Overview => cx
                .new(|cx| OverviewPage::new(&self.data, window, cx))
                .into(),
            PageUi::Settings => cx.new(|cx| SettingsPage::new(&self.data, cx)).into(),
            PageUi::ModelSettings => cx.new(|cx| ModelPage::new(&self.data, window, cx)).into(),
        };
        self.active_page = page;
        self.active_page_view = new_page;
        cx.notify();
    }
}

impl Render for FrontendUi {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let notification_layer = Root::render_notification_layer(window, cx);
        let on_page_change = |page| {
            cx.listener(move |this, _, window, cx| {
                this.change_page(page, window, cx);
            })
        };

        div()
            .flex()
            .size_full()
            .child(
                Sidebar::new(Side::Left)
                    .header(SidebarHeader::new().child("subwin"))
                    .child(
                        SidebarGroup::new("Навигация").child(
                            SidebarMenu::new()
                                .child(
                                    SidebarMenuItem::new("Главная")
                                        .active(self.active_page == PageUi::Overview)
                                        .icon(IconName::LayoutDashboard)
                                        .on_click(on_page_change(PageUi::Overview)),
                                )
                                .child(
                                    SidebarMenuItem::new("Настройки приложения")
                                        .active(self.active_page == PageUi::Settings)
                                        .icon(IconName::Settings)
                                        .on_click(on_page_change(PageUi::Settings)),
                                )
                                .child(
                                    SidebarMenuItem::new("Настройки модели")
                                        .active(self.active_page == PageUi::ModelSettings)
                                        .icon(IconName::Bot)
                                        .on_click(on_page_change(PageUi::ModelSettings)),
                                ),
                        ),
                    ),
            )
            .child(div().p_5().size_full().child(self.active_page_view.clone()))
            .children(notification_layer)
    }
}
