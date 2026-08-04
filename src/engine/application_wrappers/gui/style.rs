use iced::{Background, Border, Color};
use iced::border::Radius;
use iced::event::Status;
use iced::widget::container;

pub fn container_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        border: Border {
            color: Color::from_rgb(0.0, 0.0, 0.0), // Black border
            width: 2.0,                           // 2px wide
            radius: Radius::new(5.0),             // Rounded corners
        },
        ..container::Style::default()
    }
}


pub fn log_window_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        border: Border {
            color: Color::from_rgb(0.0, 0.0, 0.0), // Black border
            width: 1.0,                           // 2px wide
            radius: Radius::new(0),
        },
        background: Some(Background::Color(Color::BLACK)),
        ..container::Style::default()
    }
}