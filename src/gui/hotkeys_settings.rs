use eframe::egui::{Ui, Event, Context};

use crate::itc::SettingsEvent;
use crate::hotkeys::{RegisteredHotkeys, HotkeyName, self};
use eframe::egui::KeyboardShortcut;
use std::io::stderr;
use std::io::Write;
use std::sync::Arc;

use super::error_alert;

#[derive(Clone)]
pub struct HotkeysSettings
{
    registering: i32,
    alert: Option<&'static str>,
    registered_hotkeys: Arc<RegisteredHotkeys>
}

impl HotkeysSettings
{
    pub fn new(registered_hotkeys: Arc<RegisteredHotkeys>) -> Self
    {
        Self {registering:-1, alert: None, registered_hotkeys}
    }

    pub fn update(&mut self, ui: &mut Ui, ctx: &Context) -> SettingsEvent
    {
        let mut ret = SettingsEvent::Nil;

        error_alert::show_error_alert(ctx, &mut self.alert);


        //controllo se è in corso la registrazione di una hotkey
        if self.registering >= 0
        {
            if let Some(new_hk) = self.registration_phase(ui)
            {
                let str_kh = new_hk.format(&eframe::egui::ModifierNames::NAMES, std::env::consts::OS == "macos" );
                if let Err(e) = self.registered_hotkeys.register(str_kh, HotkeyName::from(self.registering as usize))
                {
                    return SettingsEvent::Error(e);
                }
                self.registering = -1;
            }
        }
        
        ui.vertical(|ui|
            {

                for i in 0..hotkeys::N_HOTK
                {
                    let mut label: String = HotkeyName::from(i).into();
                    label.push_str(": ");
                    let value = match self.registered_hotkeys.get_string(HotkeyName::from(i)) {Some(str) => str.clone(), None => String::from("")};

                    self.row_gui(ui, label, value, i);
                }

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

    fn row_gui(&mut self, ui: &mut Ui, label: String, value: String, row_n: usize)
    {
        ui.add_enabled_ui(self.registering < 0 || self.registering as usize == row_n, |ui|
        {
            ui.horizontal(|ui|
                {
                    ui.label(label);
                    ui.label(value);
                    
                    ui.with_layout(eframe::egui::Layout::right_to_left(eframe::egui::Align::TOP), |ui|
                    {   

                        if ui.button("Delete hotkey").clicked()
                        {
                            if let Err(e) = self.registered_hotkeys.unregister(HotkeyName::from(row_n))
                            {
                                self.alert.replace("Error: unable to complete the operation");
                                write!(stderr(), "Err = {}", e);
                            }
                        } 

                        
                        if ui.button("Set hotkey").clicked()
                        {
                            //avvia la registrazione della hotkey
                            self.registering = row_n as i32;
                        }
 
                    });
                    
                });
        });
        
    }

    fn registration_phase(&mut self, ui: &mut Ui) -> Option<KeyboardShortcut>
    {
        let mut ret = None;
        let events = ui.input(|i| {i.events.clone()});
        for event in &events
        {
            match event
            {
                //la prima lettera premuta termina il processo di registrazione della hotkey
                Event::Key{key, pressed: _ , modifiers, repeat}  =>  //TO DO: capire come usare pressed per migliorare la performace
                {
                      if modifiers.any() && *repeat == false
                      {
                        ret = Some(KeyboardShortcut::new(modifiers.clone(), key.clone()));
                      }else {
                          self.alert.replace("Invalid shortcut. Please press any modifier before the char. Press each button only once.");
                      }
                }
                _ => ()
            }
        }
        ret
    }


}
/*
impl eframe::App for Content {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Shortcut Registration");
            if self.setting_hotkeys {
                if ctx.input(|i| any_key_released(i, self.previous_frame_modifiers)) {
                    self.setting_hotkeys = false;
                    println!("set {}!", self.hotkey);
                    self.previous_frame_modifiers = egui::Modifiers::NONE;
                } else {
                    self.hotkey.update(
                        ctx.input(|i| i.keys_down.clone()),
                        ctx.input(|i| i.modifiers),
                    ); // todo: high cost of cloning every time
                    ui.label(self.hotkey.to_string());
                    ctx.input(|i| self.previous_frame_modifiers = i.modifiers);
                }
            } else {
                if ui.button("Set").clicked() {
                    self.setting_hotkeys = true;
                }
            }
        });
    }
}



#[derive(Default)]
struct Hotkey {
    keys: HashSet<egui::Key>,
    modifiers: egui::Modifiers,
}

impl Hotkey {
    fn update(&mut self, keys: HashSet<egui::Key>, modifiers: egui::Modifiers) {
        self.keys = keys;
        self.modifiers = modifiers;
    }
}

impl Display for Hotkey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut str = String::new();
        str.push_str(&ModifierNames::NAMES.format(&self.modifiers, false)); // todo: macOs
        for key in self.keys.iter() {
            str.push('+');
            str.push_str(key.name());
        }
        write!(f, "{str}")
    }
}

fn any_key_released(input: &egui::InputState, previous_frame_modifiers: egui::Modifiers) -> bool {
        input.events.iter().any(|event| {
            if let egui::Event::Key {
                key: _key, pressed, ..
            } = event
            {
                !*pressed
            } else {
                false
            }
        }) || !input.modifiers.contains(previous_frame_modifiers)
    }
*/