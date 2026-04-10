use gui::App;

fn main() -> iced::Result {
<<<<<<< HEAD
    iced::application(App::title, App::update, App::view)
        .theme(App::theme)
        .run_with(App::new)
=======
    iced::application(App::new, App::update, App::view)
        .title("nodeimg")
        .theme(App::theme)
        .run()
>>>>>>> 08a5b55 (feat: iced 0.14 无限画布 + 触控板捏合缩放 patch)
}
