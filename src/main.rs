use egui::{CentralPanel, Color32, Frame, Pos2, Vec2, Visuals};

fn main() -> eframe::Result<()> {
    let mut native_options = eframe::NativeOptions::default();
    native_options.transparent = true;
    native_options.initial_window_size = Some(Vec2::new(1919.0, 1080.0));
    native_options.initial_window_pos = Some(Pos2::new(0.0,0.0));
    native_options.decorated = false;
    native_options.always_on_top = true;
    eframe::run_native("My egui App", native_options, Box::new(|cc| Box::new(MyEguiApp::new(cc))))
}

#[derive(Default)]
struct MyEguiApp {}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self::default()
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut central_panel = CentralPanel::default();
        central_panel = central_panel.frame(Frame::default().fill(Color32::from_rgba_unmultiplied(255, 255, 255, 50)));
        central_panel.show(ctx, |ui| {
            ui.label("Hell");
        });
    }
    fn clear_color(&self, _visuals: &Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}