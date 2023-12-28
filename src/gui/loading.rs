use eframe::egui::CentralPanel;
use eframe::egui::Context;

/// Mostra uno spinner.
pub(crate) fn show_loading(ctx: &Context) {
    CentralPanel::default().show(ctx, |ui| {
        ui.centered_and_justified(|ui2| ui2.spinner());
    });
}
