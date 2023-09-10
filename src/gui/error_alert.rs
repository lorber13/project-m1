use eframe::egui::Window;

use crate::gui::egui::Context;
use crate::gui::egui::Pos2;
use std::sync::{Mutex, Arc};

pub fn show_error_alert(ctx: &Context, show: Option<&'static str>>>)
{
    let mut guard = show.lock().unwrap();
    if let Some(msg) = *guard
    {
        Window::new("Alert")
        .default_pos(Pos2::new(70.0, 70.0))
        .show(ctx, 
                |ui|
                                {
                                    ui.label(msg);
                                    if ui.button("Close").clicked()
                                    {
                                        *guard = None;
                                    }
                                });    
    }
    
}