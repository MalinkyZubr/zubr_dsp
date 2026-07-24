use iced::Theme;
use zubr_dsp::gui::app::{App, app_update, app_view, app_generator};

pub fn basic_gui() -> Result<(), String> {
    let _ = iced::application(app_generator::<1024>(), app_update, app_view)
        .theme(Theme::Dark)
        .exit_on_close_request(true)
        .run();
    Ok(())
}