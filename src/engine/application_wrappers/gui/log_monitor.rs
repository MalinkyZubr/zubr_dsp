use iced::widget::{button, checkbox, column, container, pick_list, row, scrollable, text};
use iced::{Element, Fill, Task};
use log::{error, info, trace, Level};
use std::borrow::Borrow;
use std::collections::VecDeque;
use iced::widget::text::Wrapping::WordOrGlyph;
use crate::engine::application_wrappers::gui::style::{container_style, log_window_style};
use crate::engine::control_plane::logging::{init_full_logger, SET_LOG_LEVEL};

#[derive(Debug, Default)]
pub struct LogMonitor<const MaxLogSize: usize> {
    logs: VecDeque<String>,
    log_level: LevelWrapper,
    logging: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LevelWrapper {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}
impl LevelWrapper {
    pub fn to_real_level(&self) -> Level {
        match self {
            LevelWrapper::Trace => Level::Trace,
            LevelWrapper::Debug => Level::Debug,
            LevelWrapper::Info => Level::Info,
            LevelWrapper::Warning => Level::Warn,
            LevelWrapper::Error => Level::Error,
        }
    }

    fn to_string_o(self) -> std::string::String {
        match self {
            LevelWrapper::Trace => String::from("trace"),
            LevelWrapper::Debug => String::from("debug"),
            LevelWrapper::Info => String::from("info"),
            LevelWrapper::Warning => String::from("warning"),
            LevelWrapper::Error => String::from("error"),
        }
    }
}
impl ToString for LevelWrapper {
    fn to_string(&self) -> std::string::String {
        match self {
            LevelWrapper::Trace => String::from("trace"),
            LevelWrapper::Debug => String::from("debug"),
            LevelWrapper::Info => String::from("info"),
            LevelWrapper::Warning => String::from("warning"),
            LevelWrapper::Error => String::from("error"),
        }
    }
}

impl Default for LevelWrapper {
    fn default() -> Self {
        LevelWrapper::Error
    }
}

#[derive(Debug, Clone)]
pub(crate) enum LogMessage {
    ToggleLogging,
    ClearLogWindow,
    SetLogLevel(LevelWrapper),
    PushLogMessages(Vec<String>)
}

impl<const MaxLogSize: usize> LogMonitor<MaxLogSize> {
    fn new() -> Self {
        Self::default()
    }
    pub fn update(&mut self, message: LogMessage) -> Task<LogMessage> {
        match message {
            LogMessage::ToggleLogging => {
                self.logging = !self.logging;
                let level_filter;
                if !self.logging {
                    error!("Logging: {}", self.logging);
                    level_filter = log::LevelFilter::Off;
                }
                else {
                    level_filter = self.log_level.to_real_level().to_level_filter()
                }
                SET_LOG_LEVEL(level_filter);
                if self.logging {
                    error!("Logging: {}", self.logging);
                }

                Task::none()
            }
            LogMessage::ClearLogWindow => {
                self.logs.clear();
                info!("Clear log window");
                Task::none()
            }
            LogMessage::SetLogLevel(level) => {
                self.log_level = level;
                error!("Log level: {:?}", level);
                SET_LOG_LEVEL(level.to_real_level().to_level_filter());
                trace!("For your information I am gooning");
                Task::none()
            }
            LogMessage::PushLogMessages(mut messages) => {
                while !messages.is_empty() {
                    let message = messages.pop().unwrap();
                    if self.logs.len() > MaxLogSize {
                        self.logs.pop_front();
                    }
                    self.logs.push_back(message);
                }

                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, LogMessage> {
        let log_string = self.logs.iter().fold(String::new(), |acc, x| acc + &x.to_string() + "\n________________________\n");
        let log_monitor = container(scrollable(text(log_string)).height(Fill).width(Fill)).style(log_window_style);
        let clear_button = button(text("Clear Logs").wrapping(WordOrGlyph).width(Fill))
            .padding(10)
            .on_press(LogMessage::ClearLogWindow);
        let enable_logging_check = checkbox(self.logging).on_toggle(|x| LogMessage::ToggleLogging);
        let enable_logging_check_text = text("Enable Logging").wrapping(WordOrGlyph);

        let levels = [
            LevelWrapper::Trace,
            LevelWrapper::Debug,
            LevelWrapper::Info,
            LevelWrapper::Warning,
            LevelWrapper::Error,
        ];
        let log_level_selector =
            pick_list(levels, Some(self.log_level), LogMessage::SetLogLevel)
                .placeholder("Log level");

        let controls = row![
            clear_button,
            enable_logging_check_text,
            enable_logging_check,
            log_level_selector
        ]
        .spacing(10);

        let content = column![log_monitor, controls].spacing(10);

        container(content).padding(20).into()
    }
}

impl<'a, const MaxLogSize: usize> From<&'a LogMonitor<MaxLogSize>> for Element<'a, LogMessage> {
    fn from(log_monitor: &'a LogMonitor<MaxLogSize>) -> Self {
        log_monitor.view()
    }
}
