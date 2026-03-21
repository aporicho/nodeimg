use eframe::egui;
use nodeimg::app;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("Node Image Studio"),
        vsync: false,
        ..Default::default()
    };

    eframe::run_native(
        "Node Image Studio",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
