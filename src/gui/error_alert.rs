
use eframe::egui::Window;
use eframe::egui::Context;
use eframe::egui::Pos2;

/// Riceve il riferimento mutabile ad una Option contenente una stringa.
/// Fintanto che la Option contiene Some(..), visualizza una nuova finestra contenente la stringa.<br>
/// Se premuto il tasto "Close", svuota la Option.
pub fn show_error_alert(ctx: &Context, show: &mut Option<String>)
{
    if let Some(msg) = show.take() {
        Window::new("Alert")
            .default_pos(Pos2::new(100.0, 100.0))
            .show(ctx, |ui| 
            {
                //if DEBUG {println!("DEBUG: alert = {}", msg);}
                ui.heading(msg.clone());

                ui.add_space(10.0);

                if !ui.button("Close").clicked() {
                    let _ = show.insert(msg);
                }
            });
    }
}
