use iced::widget::{button, checkbox, pick_list, row, scrollable, text, column, container};
use iced::{Element, Fill, Task};
use log::{info, Level};
use std::borrow::Borrow;
use iced::alignment::Vertical;
use crate::engine::application_wrappers::gui::style::container_style;
use super::log_monitor::LogMonitor;
use super::node_overview_table::PipelineTable;


#[derive(Default)]
pub struct CncWindow {

}

#[derive(Debug, Clone)]
pub enum CncMessage {
    Kill,
    Pause,
    Start
}


impl CncWindow {
    fn new() -> Self {
        Self {
        }
    }
    
    pub fn update(&mut self, message: CncMessage) -> Task<CncMessage> {
        match message {
            CncMessage::Kill => {info!("GUI: Kill message")},
            CncMessage::Pause => {info!("GUI: Pause message")},
            CncMessage::Start => {info!("GUI: Start message")}
        };
        Task::none()
    }

    pub fn view(&self) -> Element<'_, CncMessage> {
        let kill_button = button("Kill").on_press(CncMessage::Kill);
        let pause_button = button("Pause").on_press(CncMessage::Pause);
        let start_button = button("Start").on_press(CncMessage::Start);
        let window_title = text("  ZubrDSP: Control").size(20);
        
        let control_buttons = container(row![start_button, pause_button, kill_button]
            .spacing(10)
        );
        let content = container(column![window_title, control_buttons]
            .spacing(10)
        );
        
        container(content)
            .center_x(Fill)
            .padding(20)
            .into()
    }
}