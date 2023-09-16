

use eframe::egui;
use screenshots::DisplayInfo;
use super::{super::*, GlobalGuiState};
use screenshots::Screen;
extern crate image;
use super::super::itc::ScreenshotDim;
use std::rc::Rc;
use std::io::stderr;
use std::io::Write;
use std::sync::mpsc::Sender;
use eframe::egui::ColorImage;




pub struct MainWindow {
    output_format: image_coding::ImageFormat,
    area: ScreenshotDim,
    screen_id: u32
}
impl MainWindow{
    pub fn new() -> Self {
        Self {
            output_format: image_coding::ImageFormat::Png,
            area: ScreenshotDim::Fullscreen,
            screen_id: screenshot::get_main_screen_id()
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) -> Option<(ScreenshotDim, u32)> {

        
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

                egui::ComboBox::from_label("Screen") //prova di menù a tendina per scegliere se fare uno screen di tutto, oppure per selezionare un rettangolo
                    .selected_text(format!("{:?}", self.screen_id))
                    .show_ui(ui, |ui|{
                        ui.style_mut().wrap = Some(false);
                        ui.set_min_width(60.0);
                        screenshot::get_all_screens_incons(150).into_iter().for_each(
                            {
                                |s|
                                {
                                    let di = s.0.display_info;
                                    let rgba = s.1;
                                    let str = format!("id: {} ({}x{})", di.id, di.width, di.height);
                                    let txt = ctx.load_texture("icon", 
                                                                        ColorImage::from_rgba_unmultiplied(
                                                                                    [rgba.width() as usize, rgba.height() as usize],
                                                                                     rgba.as_raw(),
                                                                                ), Default::default());
                                    ui.horizontal(|ui|
                                    {
                                        ui.image(txt.id(), txt.size_vec2());
                                        ui.selectable_value(&mut self.screen_id, di.id, &str);
                                    });
                                    
                                }
                            }
                        );
                        
                    });
                    ui.end_row();

                ui.separator();
                
                // gestione della pressione del pulsante "Acquire"
                if ui.button("Acquire").clicked(){
                    //invio, tramite Channel, di un segnale al thread principale per richiedere il salvataggio dello screenshot
                    //se l'utente ha selezionato screenshot di un'area, si fa partire il processo per la selezione dell'area 
                    ret = Some((self.area.clone(), self.screen_id));
                }
            });
        ret
    }
}
  
