#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use slowspot::APP_NAME;
use std::sync::Arc;

struct ScaffoldApp;

impl ScaffoldApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "NotoSans".to_owned(),
            Arc::new(egui::FontData::from_static(include_bytes!(
                "../assets/fonts/NotoSans-Regular.ttf"
            ))),
        );

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "NotoSans".to_owned());

        cc.egui_ctx.set_fonts(fonts);

        Self
    }
}

impl eframe::App for ScaffoldApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("SlowSpot");
            ui.separator();
            ui.label("Xin chào — Tiến trình đang làm chậm máy");
            ui.label("Cửa sổ thử nghiệm kiểm tra hiển thị tiếng Việt có dấu.");
        });
    }
}

fn main() {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_title(APP_NAME),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        APP_NAME,
        native_options,
        Box::new(|cc| Ok(Box::new(ScaffoldApp::new(cc)))),
    ) {
        eprintln!("Failed to launch {APP_NAME}: {err}");
        std::process::exit(1);
    }
}
