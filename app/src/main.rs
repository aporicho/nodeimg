mod demo;
mod demo_gallery;
mod panels;

fn main() {
    gui::shell::run::<demo::DemoApp>();
}
