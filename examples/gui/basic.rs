use iced::Theme;
use zubr_dsp::engine::application_wrappers::gui::app::{App, app_update, app_view, app_generator};

pub fn basic_gui() -> Result<(), String> {
    let _ = iced::application(app_generator::<1024>(None), app_update, app_view)
        .theme(Theme::
        TokyoNightStorm
        )
        .exit_on_close_request(true)
        .run();
    Ok(())
}