mod demo;
mod demo_gallery;
mod image_demo;
mod panels;
mod workspace;

fn main() {
    gui::shell::run::<demo::DemoApp>();
}
