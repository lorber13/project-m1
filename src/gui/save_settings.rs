

use eframe::egui::{self, ScrollArea};
use crate::itc::SettingsEvent;
use chrono::Local;
use std::cell::RefCell;
use super::{file_dialog, error_alert};
use std::rc::Rc;
use std::cell::Cell;


#[derive(Clone)]
struct DefaultDir
{
    enabled: bool,
    path: String
}

#[derive(Clone, Copy)]
enum DefaultNameMode
{
    OnlyName,
    Counter(u64),
    Timestamp
}

impl Into<&'static str> for DefaultNameMode
{
    fn into(self) -> &'static str
    {
        match self
        {
            Self::OnlyName => "Default name",
            Self::Counter(..) => "Default name + incremental number",
            Self::Timestamp => "Default name + timestamp"
        }
    }
}

impl PartialEq for DefaultNameMode
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other)
        {
            (DefaultNameMode::Counter(..), DefaultNameMode::Counter(..)) | (DefaultNameMode::Timestamp, DefaultNameMode::Timestamp) => true,
            _ => false
        }
    }
}


#[derive(Clone)]
struct DefaultName
{
    enabled: bool,
    mode: Cell<DefaultNameMode>,
    name: String
}
#[derive(Clone)]
pub struct SaveSettings
{
    default_dir: DefaultDir,
    default_name: DefaultName,
    alert: Rc<RefCell<Option<&'static str>>>
}

impl SaveSettings
{
    pub fn new(alert: Rc<RefCell<Option<&'static str>>>) -> Self
    {
        Self {default_dir: DefaultDir { enabled: false, path: "".to_string() }, 
                default_name: DefaultName { enabled: false, name: "".to_string(), mode: Cell::new(DefaultNameMode::Timestamp),},
                alert
            }
    }

    pub fn update(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) -> SettingsEvent
    {
        let mut ret = SettingsEvent::Nil;

        ui.set_height(ui.available_height());
        ScrollArea::new([true, false]).show(ui, |ui|
        {
            ui.add(egui::Checkbox::new(&mut self.default_dir.enabled, "Save all screenshot in a default directory"));
            ui.add_enabled_ui(self.default_dir.enabled, |ui|
            {
                ui.horizontal(|ui|
                        {
                            ui.add(egui::TextEdit::singleline(&mut self.default_dir.path));
                            if ui.button("📁").clicked()
                            {
                                match file_dialog::show_directory_dialog(Some(&self.default_dir.path))
                                {
                                    None => (),
                                    Some(pb) => self.default_dir.path = String::from(pb.to_str().unwrap())
                                }
                            }
                        });
            });
            ui.separator();

            ui.add(egui::Checkbox::new(&mut self.default_name.enabled, "Default file name"));
            ui.add_enabled_ui(self.default_name.enabled, |ui|
            {
                ui.horizontal(|ui| {
                    let former = self.default_name.name.clone();
                    let res1 = ui.add(egui::TextEdit::singleline(&mut self.default_name.name));
                    if res1.lost_focus() && self.default_name.name != former 
                    {
                        if let DefaultNameMode::Counter(_) = self.default_name.mode.get()
                        {
                            self.default_name.mode.replace(DefaultNameMode::Counter(0));
                        }
                    }

                    egui::ComboBox::from_label("Naming Mode") //prova di menù a tendina per scegliere se fare uno screen di tutto, oppure per selezionare un rettangolo
                    .selected_text(<DefaultNameMode as Into<&'static str>>::into(self.default_name.mode.get()))
                    .show_ui(ui, |ui|{
                        ui.style_mut().wrap = Some(false);
                        ui.set_min_width(60.0);
                        ui.selectable_value(&mut self.default_name.mode.get(), DefaultNameMode::OnlyName, <DefaultNameMode as Into<&'static str>>::into(DefaultNameMode::OnlyName))
                        .on_hover_text("If exists another file with the same name in the dir, it will be overwritten.");
                        ui.selectable_value(&mut self.default_name.mode.get(), DefaultNameMode::Counter(0), <DefaultNameMode as Into<&'static str>>::into(DefaultNameMode::Counter(0)))
                        .on_hover_text("If exists another file with the same name in the dir, it will be overwritten.");
                        ui.selectable_value(&mut self.default_name.mode.get(), DefaultNameMode::Timestamp, <DefaultNameMode as Into<&'static str>>::into(DefaultNameMode::Timestamp))
                        .on_hover_text("timestamp format: YYYY-MM-DD_HH-MM-SS");
                    });
                    
                });


            });
            ui.separator();


            ui.add_space(30.0);
            ui.horizontal(|ui|
                {
                    if ui.button("Save").clicked() {
                        if self.default_dir.enabled && ( self.default_dir.path.len() == 0 || !std::path::Path::new(&self.default_dir.path).exists())
                        {
                            if crate::DEBUG {println!("Found an invalid dir path");}
                            self.alert.borrow_mut().replace("invalid default directory path");
                        }else {
                            ret = SettingsEvent::Saved;
                        }
                    }
                    if ui.button("Abort").clicked() {ret = SettingsEvent::Aborted;}
                })
        
        });


        ret

    }

    ///returns false in case default dir is not enabled
    pub fn get_default_dir(&self) -> Option<String>
    {
        if !self.default_dir.enabled || self.default_dir.path.len() == 0 {return None;}

        Some(self.default_dir.path.clone())
    }

    ///Returns None if default name is not enabled.
    /// It automatically increments the internal counter.
    pub fn get_default_name(&self) -> Option<String>
    {
        if !self.default_name.enabled {return None;}
    
        match self.default_name.mode.get()
        {
            DefaultNameMode::OnlyName =>
            {
                Some(self.default_name.name.clone())
            },

            DefaultNameMode::Counter(c) => 
            {
                let str = format!("{}{}", self.default_name.name, c);
                self.default_name.mode.replace(DefaultNameMode::Counter(c+1));
                Some(str)
            },

            DefaultNameMode::Timestamp =>
            {
                const TIMESTAMP_FMT: &'static str = "%Y-%m-%d_%H%M%S";
                let str = format!("{}{}", self.default_name.name, Local::now().format(TIMESTAMP_FMT));
                Some(str)
            }
        }

        
    }
}
