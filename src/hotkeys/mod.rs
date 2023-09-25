use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyEventReceiver, GlobalHotKeyManager};
use global_hotkey::hotkey::HotKey;

pub struct RegisteredHotkeys
{
    fullscreen_screenshot: Option<HotKey>, //TO DO: implementare Dysplay per HotKey
    rect_screenshot: Option<HotKey>
}


impl RegisteredHotkeys
{
    pub fn set_fullscreen_hotkey(&mut self, h: HotKey)
    {
        self.fullscreen_screenshot = Some(h);
    }

    pub fn set_rect_hotkey(&mut self, h: HotKey)
    {
        self.rect_screenshot = Some(h);
    }


}


fn main() -> Result<(), eframe::Error> {
    thread::spawn(|| {
            loop {
                if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv()
                {
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