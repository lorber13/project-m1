

use eframe::egui;
use eframe::epaint::TextureId;
use super::{super::*, GlobalGuiState};
use screenshot::ScreensManager;
extern crate image;
use super::super::itc::ScreenshotDim;
use std::rc::Rc;
use std::io::stderr;
use std::io::Write;
use std::sync::mpsc::Sender;
use eframe::egui::ColorImage;
use eframe::egui::Vec2;

#[derive(Clone, Copy)]
pub struct Delay {
    pub delayed: bool,
    pub scalar: f64,
}

pub struct MainWindow {
    area: ScreenshotDim,
    delay: Delay,
}
impl MainWindow{
    pub fn new() -> Self {
        Self {
            area: ScreenshotDim::Fullscreen,
            delay: Delay{delayed: false, scalar: 0.0 },
        }
    }

    pub fn update(&mut self, screens_manager: &mut ScreensManager, ctx: &egui::Context, _frame: &mut eframe::Frame) -> Option<(ScreenshotDim, Delay)> {

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


                ui.separator();

                ui.horizontal(|ui|
                {
                    egui::ComboBox::from_label("Screen") //prova di menù a tendina per scegliere se fare uno screen di tutto, oppure per selezionare un rettangolo
                    .selected_text(format!("{:?}", screens_manager.curr_screen_index))
                    .show_ui(ui, |ui|{
                        ui.style_mut().wrap = Some(false);
                        ui.set_min_width(60.0);
                        for (i, s) in screens_manager.screens.iter().enumerate()
                            {
                                let di = s.0.display_info;
                                    let str = format!("({}x{})", di.width, di.height);
                                    
                                    ui.horizontal(|ui|
                                    {
                                        if let Ok(guard) = s.1.try_lock()
                                        {
                                            if let Some(rgba) = guard.clone()
                                            {
                                                let txt = ctx.load_texture("icon", 
                                                                            ColorImage::from_rgba_unmultiplied(
                                                                                        [rgba.width() as usize, rgba.height() as usize],
                                                                                        rgba.as_raw(),
                                                                                    ), Default::default());
                                                ui.image(txt.id(), txt.size_vec2());
                                            }else {
                                                ui.spinner();
                                            }
                                        }else {ui.spinner();}
                                        
                                        ui.selectable_value(&mut screens_manager.curr_screen_index, i, &str);
                                    });
                                    
                            }
                    });

                    if ui.button("↺").on_hover_text("Refresh").clicked()
                    {
                        screens_manager.update_available_screens();
                    }
                });

                    

                ui.separator();

                ui.add(egui::Checkbox::new(&mut self.delay.delayed, "Timer"));
                if self.delay.delayed {
                    ui.add(egui::Slider::new(&mut self.delay.scalar, 0.0..=5.0));
                }

                ui.separator();
                // gestione della pressione del pulsante "Acquire"
                if ui.button("Acquire").clicked(){
                    //invio, tramite Channel, di un segnale al thread principale per richiedere il salvataggio dello screenshot
                    //se l'utente ha selezionato screenshot di un'area, si fa partire il processo per la selezione dell'area 
                    ret = Some((self.area.clone(), self.delay));
                }
            });
        ret
    }
}
  
