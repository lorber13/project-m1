use std::{env, thread};
use eframe::egui;
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager};
use global_hotkey::hotkey::{Code, Modifiers};


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
}

impl WaylandContent {
    fn new() -> Self {
        todo!()
    }
}

impl eframe::App for WaylandContent {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            todo!()
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