
use eframe::egui::Window;
use eframe::egui::Context;
use eframe::egui::Pos2;

use crate::DEBUG;

/// Riceve il riferimento mutabile ad una Option contenente una stringa.
/// Fintanto che la Option contiene Some(..), visualizza una nuova finestra contenente la stringa.<br>
/// Se premuto il tasto "Close", svuota la Option.
pub fn show_error_alert(ctx: &Context, show: &mut Option<&'static str>)
{
    if let Some(msg) = *show {
        Window::new("Alert")
            .default_pos(Pos2::new(100.0, 100.0))
            .show(ctx, |ui| 
            {
                //if DEBUG {println!("DEBUG: alert = {}", msg);}
                ui.heading(msg);

                ui.add_space(10.0);

                if ui.button("Close").clicked() {
                    *show = None;
                }
            });
    }
}
