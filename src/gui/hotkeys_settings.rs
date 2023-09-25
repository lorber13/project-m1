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
