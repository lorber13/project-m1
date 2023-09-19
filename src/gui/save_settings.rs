

use eframe::egui;
use crate::itc::SettingsEvent;

use super::{file_dialog, error_alert};


#[derive(Clone)]
struct DefaultDir
{
    enabled: bool,
    path: String
}
#[derive(Clone)]
pub struct SaveSettings
{
    default_dir: DefaultDir,
    alert: Option<&'static str>
}

impl SaveSettings
{
    pub fn new() -> Self
    {
        Self {default_dir: DefaultDir { enabled: false, path: "".to_string() }, 
                alert: None}
    }

    pub fn update(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, frame: &mut eframe::Frame) -> SettingsEvent
    {
        let mut ret = SettingsEvent::Nil;

        egui::CentralPanel::default().show_inside(ui, |ui|
        {
            error_alert::show_error_alert(ctx, &mut self.alert);
            ui.add_enabled_ui(self.alert==None, |ui|
            {
                ui.add(egui::Checkbox::new(&mut self.default_dir.enabled, "Save all screenshot in a default directory"));
                ui.add_enabled_ui(self.default_dir.enabled, |ui|
                {
                    ui.horizontal(|ui|
                            {
                                let res1 = ui.add(egui::TextEdit::singleline(&mut self.default_dir.path));
                                if ui.button("📁").clicked()
                                {
                                    match file_dialog::show_directory_dialog(&self.default_dir.path)
                                    {
                                        None => (),
                                        Some(pb) => self.default_dir.path = String::from(pb.to_str().unwrap())
                                    }
                                }
                            });
                });
                ui.separator();
                ui.horizontal(|ui|
                    {
                        if ui.button("Save").clicked() {
                            if self.default_dir.enabled && ( self.default_dir.path.len() == 0 || !std::path::Path::new(&self.default_dir.path).exists())
                            {
                                self.alert = Some("invalid default directory path");
                            }else {
                                ret = SettingsEvent::Saved;
                            }
                        }
                        if ui.button("Abort").clicked() {ret = SettingsEvent::Aborted;}
                    })
            });
            
        });

        ret

    }
}
