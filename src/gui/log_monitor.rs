use iced::widget::{button, checkbox, column, container, pick_list, row, scrollable, text};
use iced::{Element, Fill, Task};
use log::Level;
use std::borrow::Borrow;
use iced::widget::text::Wrapping::WordOrGlyph;
use crate::gui::style::{container_style, log_window_style};

#[derive(Debug, Default)]
pub struct LogMonitor<const MaxLogSize: usize> {
    log_string: String,
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

#[derive(Debug, Clone)]
pub(crate) enum LogMessage {
    ToggleLogging,
    ClearLogWindow,
    SetLogLevel(LevelWrapper),
}

impl<const MaxLogSize: usize> LogMonitor<MaxLogSize> {
    fn new() -> Self {
        Self::default()
    }
    fn push_log_message(&mut self, message: String) {
        if self.logging {
            self.log_string.push_str(&message);
            if self.log_string.len() > MaxLogSize {
                self.log_string
                    .drain(0..(self.log_string.len() - MaxLogSize));
            }
        }
    }
    pub fn update(&mut self, message: LogMessage) -> Task<LogMessage> {
        match message {
            LogMessage::ToggleLogging => {
                self.logging = !self.logging;
                Task::none()
            }
            LogMessage::ClearLogWindow => {
                self.log_string.clear();
                Task::none()
            }
            LogMessage::SetLogLevel(level) => {
                self.log_string = String::from(level.to_string());
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, LogMessage> {
        let log_monitor = container(scrollable(text(self.log_string.clone())).height(Fill).width(Fill)).style(log_window_style);
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
            pick_list(levels, Some(LevelWrapper::Debug), LogMessage::SetLogLevel)
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
