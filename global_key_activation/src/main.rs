use std::{env, thread};
use eframe::egui;
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager};
use global_hotkey::hotkey::{Code, Modifiers, self};


fn main() -> Result<(), eframe::Error> {
    let wayland = env::var("WAYLAND_DISPLAY").is_ok();

    let options = eframe::NativeOptions::default();

    if wayland {
        eframe::run_native(
            "My egui App",
            options,
            Box::new(|_cc| Box::<WaylandContent>::new(WaylandContent::new())),
        )
    } else {
        // event loop, listening for the hotkeys
        thread::spawn(|| {
            loop {
                if let Ok(event) = GlobalHotKeyEvent::receiver().recv() {
                    println!("tray event: {event:?}");
                }
            }
        });

        eframe::run_native(
            "My egui App",
            options,
            Box::new(|_cc| Box::<DefaultContent>::new(DefaultContent::new())),
        )
    }
}
struct WaylandContent {
    hook: livesplit_hotkey::Hook,
    hotkey: livesplit_hotkey::Hotkey,
    keycode: livesplit_hotkey::KeyCode
}

impl WaylandContent {
    fn new() -> Self {
        let hook = livesplit_hotkey::Hook::new().unwrap();
        let keycode = livesplit_hotkey::KeyCode::Digit0;
        let hotkey = livesplit_hotkey::Hotkey::from(keycode);
        hook.register(hotkey, || {
            println!("hotkey");
        }).unwrap();
        WaylandContent {
            hook,
            hotkey,
            keycode
        }
    }
}

impl eframe::App for WaylandContent {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.radio_value(&mut self.keycode, livesplit_hotkey::KeyCode::Digit0, livesplit_hotkey::KeyCode::Digit0.name());
            ui.radio_value(&mut self.keycode, livesplit_hotkey::KeyCode::Digit1, livesplit_hotkey::KeyCode::Digit1.name());
            ui.radio_value(&mut self.keycode, livesplit_hotkey::KeyCode::Digit2, livesplit_hotkey::KeyCode::Digit2.name());
            ui.radio_value(&mut self.keycode, livesplit_hotkey::KeyCode::Digit3, livesplit_hotkey::KeyCode::Digit3.name());
            ui.radio_value(&mut self.keycode, livesplit_hotkey::KeyCode::Digit4, livesplit_hotkey::KeyCode::Digit4.name());
            ui.radio_value(&mut self.keycode, livesplit_hotkey::KeyCode::Digit5, livesplit_hotkey::KeyCode::Digit5.name());
            ui.radio_value(&mut self.keycode, livesplit_hotkey::KeyCode::Digit6, livesplit_hotkey::KeyCode::Digit6.name());
            ui.radio_value(&mut self.keycode, livesplit_hotkey::KeyCode::Digit7, livesplit_hotkey::KeyCode::Digit7.name());
            ui.radio_value(&mut self.keycode, livesplit_hotkey::KeyCode::Digit8, livesplit_hotkey::KeyCode::Digit8.name());
            ui.radio_value(&mut self.keycode, livesplit_hotkey::KeyCode::Digit9, livesplit_hotkey::KeyCode::Digit9.name());
            if ui.button("set").clicked() {
                self.hook.unregister(self.hotkey).unwrap();
                self.hotkey = livesplit_hotkey::Hotkey::from(self.keycode);
                self.hook.register(self.hotkey, || {
                    println!("hotkey");
                }).expect("msg");
            }
        });
    }
}

struct DefaultContent {
    manager: GlobalHotKeyManager,
    hotkey: HotKey,
    modifier: Modifiers,
    key: Code
}

impl DefaultContent {
    fn new() -> Self {
        let manager = GlobalHotKeyManager::new().unwrap();
        let hotkey = HotKey::new(Some(Modifiers::SHIFT), Code::KeyD);
        manager.register(hotkey).unwrap();
        DefaultContent {
            manager,
            hotkey,
            modifier: Default::default(),
            key: Default::default(),
        }
    }
}

impl Default for DefaultContent {
    fn default() -> Self {
        Self {
            manager: GlobalHotKeyManager::new().unwrap(),
            hotkey: HotKey::new(Some(Modifiers::SHIFT), Code::KeyD),
            modifier: Modifiers::SHIFT,
            key: Code::KeyD
        }
    }
}

impl eframe::App for DefaultContent {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.key, Code::Digit0, "0");
                ui.radio_value(&mut self.key, Code::Digit1, "1");
                ui.radio_value(&mut self.key, Code::Digit2, "2");
                ui.radio_value(&mut self.key, Code::Digit3, "3");
                ui.radio_value(&mut self.key, Code::Digit4, "4");
                ui.radio_value(&mut self.key, Code::Digit5, "5");
                ui.radio_value(&mut self.key, Code::Digit6, "6");
                ui.radio_value(&mut self.key, Code::Digit7, "7");
                ui.radio_value(&mut self.key, Code::Digit8, "8");
                ui.radio_value(&mut self.key, Code::Digit9, "9");
            });
            if ui.button("set").clicked() {
                self.manager.unregister(self.hotkey).unwrap();
                self.hotkey = HotKey::new(None, self.key);
                self.manager.register(self.hotkey).unwrap();
            }
        });
    }
}