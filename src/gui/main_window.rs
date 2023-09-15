

use eframe::egui;
use super::{super::*, GlobalGuiState};
use screenshots::Screen;
extern crate image;
use super::super::itc::ScreenshotDim;
use std::rc::Rc;
use std::io::stderr;
use std::io::Write;
use std::sync::mpsc::Sender;




pub struct MainWindow {
    output_format: image_coding::ImageFormat,
    area: ScreenshotDim
}
impl MainWindow{
    pub fn new() -> Self {
        Self {
            output_format: image_coding::ImageFormat::Png,
            area: ScreenshotDim::Fullscreen
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) -> Option<ScreenshotDim> {

        _frame.set_decorations(true);
        _frame.set_fullscreen(false);
        _frame.set_maximized(false);
        _frame.set_window_size(egui::Vec2::new(500.0, 300.0));
        _frame.set_visible(true);
        let screens= Screen::all().expect("Mismatching type in Vec<Screen>");
        let mut ret = None;

           egui::CentralPanel::default().show(ctx, |ui|
            {
                ui.label("Capture Mode");
                ui.separator();
                egui::ComboBox::from_label("Area") //prova di menù a tendina per scegliere se fare uno screen di tutto, oppure per selezionare un rettangolo
                    .selected_text(format!("{:?}", self.area))
                    .show_ui(ui, |ui|{
                        ui.style_mut().wrap = Some(false);
                        ui.set_min_width(60.0);
                        ui.selectable_value(&mut self.area, ScreenshotDim::Fullscreen, "Full Screen");
                        ui.selectable_value(&mut self.area, ScreenshotDim::Rectangle, "Rectangle");
                    });
                    ui.end_row();

                ui.separator();
                
                // gestione della pressione del pulsante "Acquire"
                if ui.button("Acquire").clicked(){
                    //invio, tramite Channel, di un segnale al thread principale per richiedere il salvataggio dello screenshot
                    //se l'utente ha selezionato screenshot di un'area, si fa partire il processo per la selezione dell'area 
                    ret = Some(self.area.clone());
                }
            });
        ret
    }
}
  
