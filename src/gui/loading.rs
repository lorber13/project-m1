use eframe::egui::CentralPanel;
use crate::gui::egui::Context;

pub(crate) fn show_loading(ctx: &Context) {
    CentralPanel::default().show(ctx, |ui| {
        ui.centered_and_justified(ui.spinner());
    });
}