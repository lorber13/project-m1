use eframe::egui::Ui;

use crate::itc::SettingsEvent;

#[derive(Clone)]
pub struct HotkeysSettings
{
    registering: i32,
    alert: Option<&'static str>
}

impl HotkeysSettings
{
    pub fn new() -> Self
    {
        Self {registering:-1, alert: None}
    }

    pub fn update(&mut self, ui: &mut Ui) -> SettingsEvent
    {
        let mut ret = SettingsEvent::Nil;
        ui.vertical(|ui|
            {

                self.row_gui(ui, "fullscreen screenshot: ", 0);
                self.row_gui(ui, "rect screenshot: ", 1);

                ui.separator();
                ui.horizontal(|ui|
                    {
                        if ui.button("Save").clicked() {
                            if self.registering >= 0
                            {
                                self.alert = Some("Invalid operation. Please press done and then proceed");
                            }else {
                                ret = SettingsEvent::Saved;
                            }
                        }
                        if ui.button("Abort").clicked() {ret = SettingsEvent::Aborted;}
                    })
            });
        ret
    }

    fn row_gui(&mut self, ui: &mut Ui, label: &'static str, row_n: i32)
    {
        ui.add_enabled_ui(self.registering < 0 || self.registering == row_n, |ui|
        {
            ui.horizontal(|ui|
                {
                    ui.label(label);

                    ui.with_layout(eframe::egui::Layout::right_to_left(eframe::egui::Align::TOP), |ui| {
                        ui.add_enabled_ui(self.registering == row_n, |ui|
                            {
                                if ui.button("Reset").clicked()
                                {
                                    self.registering = -1;
                                }
                                if ui.button("Done").clicked() 
                                {
                                    self.registering = -1;
                                }
                                
                            });
                        
                        if ui.button("Set hotkey").clicked()
                        {
                            //avvia la registrazione della hotkey
                            self.registering = row_n;
                        }  
                    
                    });
                    
                });
        });
        
    }

}