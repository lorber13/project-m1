use eframe::egui::Window;

use eframe::egui::Context;
use eframe::egui::Pos2;

pub fn show_error_alert(ctx: &Context, show: &mut Option<&'static str>)
{
    if let Some(msg) = *show {
        Window::new("Alert")
            .default_pos(Pos2::new(70.0, 70.0))
            .show(ctx, |ui| {
                ui.label(msg);
                if ui.button("Close").clicked() {
                    *show = None;
                }
            });
    }
}
