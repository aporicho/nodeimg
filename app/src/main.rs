mod app_shell;
mod image_demo;
mod panels;
mod workspace;

fn main() {
    gui::shell::run::<app_shell::AppShell>();
}
