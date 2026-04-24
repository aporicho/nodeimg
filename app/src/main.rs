mod app_shell;
mod developer_mode;
mod image_demo;
mod panels;
mod user_mode;
mod visual_audit;
mod workspace;

fn main() {
    gui::shell::run::<app_shell::AppShell>();
}
