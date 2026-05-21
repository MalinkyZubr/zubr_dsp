use iced::widget::{button, text};
use iced::{window, Element};


fn update(counter: &mut u64, message: Message) {
    match message {
        Message::Increment => *counter += 1,
    }
}


fn view(counter: &u64) -> Element<'_, Message> {
    button(text(counter)).on_press(Message::Increment).into()
}


#[derive(Debug, Clone)]
enum Message {
    Increment,
}


pub fn basic_gui() -> Result<(), String> {
    let icon = window::icon::from_file_data(
        include_bytes!("../../assets/favicon.ico"),
        None,
    ).expect("Failed to load icon");
    iced::application(u64::default, update, view)
        .window(window::Settings {
            icon: Some(icon),
            ..Default::default()
        }).run();

    Ok(())
}