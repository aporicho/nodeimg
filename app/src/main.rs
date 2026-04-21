mod demo;
mod demo_gallery;
mod image_demo;
mod node_palette;
mod panels;

fn main() {
    gui::shell::run::<demo::DemoApp>();
}
