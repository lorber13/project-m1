

use eframe::egui;
use super::screens_manager::ScreensManager;
extern crate image;
use super::super::itc::{Delay,ScreenshotDim};
use eframe::egui::ColorImage;
use std::sync::Arc;


pub struct CaptureMode {
    area: ScreenshotDim,
    delay: Delay,
}
impl CaptureMode{
    pub fn new() -> Self {
        Self {
            area: ScreenshotDim::Fullscreen,
            delay: Delay{delayed: false, scalar: 0.0 },
        }
    }

    pub fn update(&mut self, ui: &mut egui::Ui, screens_mgr: Arc<ScreensManager>, ctx: &egui::Context, _frame: &mut eframe::Frame) -> Option<(ScreenshotDim, f64)> 
    {
        let mut ret = None;

            ui.style_mut().animation_time = 0.0;
           egui::CentralPanel::default().show_inside(ui, |ui|
            {
                ui.style_mut().animation_time = 0.0;
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

                
                self.screens_combobox(ui,  screens_mgr, ctx);
                    

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
                    ret = Some((self.area.clone(), self.delay.scalar));
                }

            });
        ret
    }

    fn screens_combobox(&self, ui: &mut egui::Ui, screens_manager: Arc<ScreensManager>, ctx: &egui::Context,)
    {
        ui.horizontal(|ui|
            {
                egui::ComboBox::from_label("Screen") //prova di menù a tendina per scegliere se fare uno screen di tutto, oppure per selezionare un rettangolo
                .selected_text(format!("{:?}", screens_manager.get_current_screen_index()))
                .show_ui(ui, |ui|{
                    ui.style_mut().wrap = Some(false);
                    ui.set_min_width(60.0);
                    for (i, s) in screens_manager.get_screens().iter().enumerate()
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
                                    
                                    let mut curr = screens_manager.get_current_screen_index();
                                    ui.selectable_value(&mut curr, i, &str);
                                    screens_manager.select_screen(curr);
                                });
                                
                        }
                });

                if ui.button("↺").on_hover_text("Refresh").clicked()
                {
                    screens_manager.update_available_screens();
                }
            });
    }
}
  
