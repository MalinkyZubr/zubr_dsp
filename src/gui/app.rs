use super::cnc_window::{CncMessage, CncWindow};
use super::log_monitor::{LogMessage, LogMonitor};
use super::node_overview_table::{PipelineTable, PipelineTableMessage};
use crate::engine::control_plane::pipeline_analytics::NodeAnalytics;
use crate::engine::data_plane::structural::generic_pipeline_node::{NodeState, RunModel};
use crate::gui::style::container_style;
use iced::alignment::Horizontal;
use iced::widget::rule::FillMode::Percent;
use iced::widget::{
    button, center_x, checkbox, column, container, pick_list, responsive, row, scrollable, text,
};
use iced::Length::Fixed;
use iced::{Center, Element, Fill, FillPortion, Shrink, Size, Task};
use log::{info, Level};

pub struct App<const MAX_LOG_MESSAGES: usize> {
    cnc_window: CncWindow,
    log_monitor: LogMonitor<MAX_LOG_MESSAGES>,
    node_overview_table: PipelineTable,
}

#[derive(Debug, Clone)]
pub enum AppMessage {
    CncWindow(CncMessage),
    LogMonitor(LogMessage),
    PipelineTable(PipelineTableMessage),
}

impl<const MAX_LOG_MESSAGES: usize> App<MAX_LOG_MESSAGES> {
    pub fn new() -> Self {
        Self {
            cnc_window: CncWindow::default(),
            log_monitor: LogMonitor::default(),
            node_overview_table: PipelineTable::new(vec![]),
        }
    }

    pub fn update(&mut self, message: AppMessage) -> Task<AppMessage> {
        match message {
            AppMessage::CncWindow(msg) => {
                let _ = self.cnc_window.update(msg);
                Task::none()
            }
            AppMessage::LogMonitor(msg) => {
                let _ = self.log_monitor.update(msg);
                Task::none()
            }
            AppMessage::PipelineTable(msg) => {
                let _ = self.node_overview_table.update(msg);
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<AppMessage> {
        responsive(|size: Size| {
            let mut overview_table_view = container(
                self.node_overview_table
                    .view()
                    .map(AppMessage::PipelineTable),
            )
            .style(container_style);

            let mut log_view = container(self.log_monitor.view().map(AppMessage::LogMonitor))
                .style(container_style);

            let mut cnc_view = container(self.cnc_window.view().map(AppMessage::CncWindow))
                .center_x(Fill)
                .style(container_style);

            info!("{}", size.height);
            let min_height_window = 600.0;
            let min_width_window = 1715.0;
            if size.width < min_width_window {
                log_view = log_view.width(Fixed(min_width_window * 0.25));
                overview_table_view = overview_table_view.width(Fixed(min_width_window * 0.75));
            } else {
                log_view = log_view.width(FillPortion(1));
                overview_table_view = overview_table_view.width(FillPortion(3));
            }

            let mut content = row![log_view, overview_table_view].spacing(10).padding(20);

            cnc_view = cnc_view.height(Shrink);
            if size.height < min_height_window {
                content = content.height(Fixed(min_height_window * 0.8));
            } else {
                content = content.height(Fill);
            }

            let app_view = column![cnc_view.width(Shrink), content]
                .spacing(10)
                .padding(10)
                .align_x(Center);

            let container = container(app_view);
            if size.width < min_width_window && size.height < min_height_window {
                let scroll_bar = scrollable::Scrollbar::new().width(10).scroller_width(8);
                scrollable(container)
                    .direction(scrollable::Direction::Both {
                        vertical: scroll_bar,
                        horizontal: scroll_bar,
                    })
                    .into()
            } else if size.width < min_width_window {
                let scroll_bar = scrollable::Scrollbar::new().width(10).scroller_width(8);
                scrollable(container)
                    .direction(scrollable::Direction::Horizontal(scroll_bar))
                    .into()
            } else if size.height < min_height_window {
                let scroll_bar = scrollable::Scrollbar::new().width(10).scroller_width(8);
                scrollable(container)
                    .direction(scrollable::Direction::Vertical(scroll_bar))
                    .into()
            } else {
                container.into()
            }
        })
        .into()
    }
}

pub fn app_generator<const MAX_LOG_MESSAGES: usize>() -> impl Fn() -> App<MAX_LOG_MESSAGES> {
    || App::new()
}

pub fn app_update<const MAX_LOG_MESSAGES: usize>(
    app: &mut App<MAX_LOG_MESSAGES>,
    message: AppMessage,
) -> Task<AppMessage> {
    app.update(message)
}

pub fn app_view<const MAX_LOG_MESSAGES: usize>(app: &App<MAX_LOG_MESSAGES>) -> Element<AppMessage> {
    app.view()
}
