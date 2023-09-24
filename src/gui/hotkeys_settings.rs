use std::env;
use eframe::egui::Ui;
use crate::itc::

#[derive(Clone)]
pub struct HotekysSettings
{
}

impl HotekysSettings
{
    pub fn new() -> Self
    {
        Self {}
    }

    pub fn update(&mut self, ui: &mut Ui)
    {
        ui.vertical(|ui|
            {

                row_gui(ui, "fullscreen screenshot: ");
                row_gui(ui, "rect screenshot: ");

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
    }

    fn row_gui(ui: &mut Ui, label: &'static str)
    {
        ui.horizontal(|ui|
            {
                let mut save_abort_enabled = false;
                ui.label(label);
                if ui.button("Set hotkey").clicked()
                {
                    //avvia la registrazione della hotkey
                    save_abort_enabled = true;
                }

                ui.add_enabled_ui(save_abort_enabled, |ui|
                    {
                        if ui.button("Done").clicked()
                        {

                        }
                        if ui.button("Reset").clicked()
                        {

                        }
                    })
            });
    }

    ///Alcune combinazioni di tasti non sono valide perchè già usate dal sistema per altri scopi
    fn validate_hotkey(h: Vec<String>) -> bool
    {
        match env::consts::OS
        {
            "linux" => validate_hotkey_linux(h),
            "macos" => validate_hotey_macos(h),
            "windows" => validate_hotkey_windows(h),
            _ => false
        }
    }

    fn validate_hotkey_linux(h: Vec<String>) -> bool
    {
        false
    }

    fn validate_hotkey_macos(h: Vec<String>) -> bool
    {
        false
    }

    fn validate_hotkey_windows(h: Vec<String>) -> bool
    {
        false
    }
}