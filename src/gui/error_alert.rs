use eframe::egui::Window;

use crate::gui::egui::Context;
use crate::gui::egui::Pos2;
use std::rc::Rc;
use std::cell::Cell;

pub fn show_error_alert(ctx: &Context, show: Rc<Cell<Option<&'static str>>>)
{
    if let Some(msg) = show.get()
    {
        Window::new("Alert")
        .default_pos(Pos2::new(70.0, 70.0))
        .show(ctx, 
                |ui|
                                {
                                    ui.label(msg);
                                    if ui.button("Close").clicked()
                                    {
                                        show.replace(None);
                                    }
                                });    
    }
    
}