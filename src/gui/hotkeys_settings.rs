use eframe::egui::{Ui, Context, Event};

use crate::itc::SettingsEvent;
use crate::hotkeys::{RegisteredHotkeys, HotkeyName, self};
use eframe::egui::KeyboardShortcut;

#[derive(Clone)]
pub struct HotkeysSettings
{
    registering: i32,
    alert: Option<&'static str>,
}

impl HotkeysSettings
{
    pub fn new() -> Self
    {
        Self {registering:-1, alert: None}
    }

    pub fn update(&mut self, ui: &mut Ui, registered_hotkeys: &mut RegisteredHotkeys, ctx: &Context) -> SettingsEvent
    {
        //controllo se è in corso la registrazione di una hotkey
        if self.registering >= 0
        {
            if let Some(new_hk) = self.registration_phase(ui, ctx)
            {
                let str_kh = new_hk.format(&eframe::egui::ModifierNames::NAMES, std::env::consts::OS == "macos" );
                registered_hotkeys.register(str_kh, HotkeyName::from(self.registering as usize));
                self.registering = -1;
            }
        }
        let mut ret = SettingsEvent::Nil;
        ui.vertical(|ui|
            {

                for i in 0..hotkeys::N_HOTK
                {
                    let mut label: String = HotkeyName::from(i).into();
                    label.push_str(": ");
                    let value = match registered_hotkeys.get_string(HotkeyName::from(i)) {Some(str) => str.clone(), None => String::from("")};

                    ui.horizontal(|ui|
                    {
                        self.row_gui(ui, label, value, i, ctx);
                        if ui.button("Delete hotkey").clicked()
                        {
                            registered_hotkeys.unregister(HotkeyName::from(i));
                        }
                    });
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

    fn row_gui(&mut self, ui: &mut Ui, label: String, value: String, row_n: usize, ctx: &Context)
    {
        ui.add_enabled_ui(self.registering < 0 || self.registering as usize == row_n, |ui|
        {
            ui.horizontal(|ui|
                {
                    ui.label(label);
                    ui.label(value);
                    
                        
                        if ui.button("Set hotkey").clicked()
                        {
                            //avvia la registrazione della hotkey
                            self.registering = row_n as i32;
                        }  
                    
                    
                });
        });
        
    }

    fn registration_phase(&mut self, ui: &mut Ui, ctx: &Context) -> Option<KeyboardShortcut>
    {
        let mut ret = None;
        let events = ui.input(|i| {i.events.clone()});
        for event in &events
        {
            match event
            {
                //la prima lettera premuta termina il processo di registrazione della hotkey
                Event::Key{key, pressed, modifiers, repeat}  =>  //TO DO: capire come usare pressed per migliorare la performace
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