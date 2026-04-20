mod demo;
mod demo_gallery;
mod image_demo;
mod panels;

fn main() {
    gui::shell::run::<demo::DemoApp>();
}
