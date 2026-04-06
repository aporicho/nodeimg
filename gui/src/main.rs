use gui::App;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("nodeimg")
        .theme(App::theme)
        .run()
}
