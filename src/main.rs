mod core;
mod database;
mod gui;
use crate::gui::GostPassApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 650.0])
            .with_min_inner_size([700.0, 500.0])
            .with_title("GostPass - Безопасное хранилище паролей"),
        ..Default::default()
    };
    
    eframe::run_native(
        "GostPass",
        options,
        Box::new(|_cc| Box::new(GostPassApp::default())),
    )
}