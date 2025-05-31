// Declare the modules we've created
mod app;
mod settings_panel;
mod graph_view;

// Use the app structure from the app module
use app::BasicApp;
use eframe::NativeOptions;

fn main() {
    let native_options = NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "egui_basic_standalone - 可交互图应用", // Window title
        native_options,
        Box::new(|cc| Ok(Box::new(BasicApp::new(cc)))),
    );
}