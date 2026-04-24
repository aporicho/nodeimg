mod app_shell;
mod image_demo;
mod panels;
mod user_mode;
mod workspace;

fn main() {
    gui::shell::run::<app_shell::AppShell>();
}
