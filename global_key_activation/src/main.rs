use eframe::{NativeOptions, run_simple_native};
use egui::{CentralPanel};
use livesplit_hotkey::{Hook, Hotkey, KeyCode};

struct Modifiers {
    ctrl: bool,
    alt: bool,
    shift: bool,
    meta: bool
}

fn main() -> eframe::Result<()> {
    let native_options = NativeOptions::default();

    let mut modifiers = Modifiers {
        ctrl: false,
        alt: false,
        shift: false,
        meta: false
    };
    let mut hotkey = Hotkey::from(KeyCode::Digit0);
    let hook = Hook::new().unwrap();

    run_simple_native("Hotkeys Prototype", native_options, move |ctx, _frame| {
        CentralPanel::default().show(ctx, |ui| {
            ui.radio_value(&mut hotkey.key_code, KeyCode::Digit0, KeyCode::Digit0.name());
            ui.radio_value(&mut hotkey.key_code, KeyCode::Digit1, KeyCode::Digit1.name());
            ui.radio_value(&mut hotkey.key_code, KeyCode::Digit2, KeyCode::Digit2.name());
            ui.radio_value(&mut hotkey.key_code, KeyCode::Digit3, KeyCode::Digit3.name());
            ui.radio_value(&mut hotkey.key_code, KeyCode::Digit4, KeyCode::Digit4.name());
            ui.radio_value(&mut hotkey.key_code, KeyCode::Digit5, KeyCode::Digit5.name());
            ui.radio_value(&mut hotkey.key_code, KeyCode::Digit6, KeyCode::Digit6.name());
            ui.radio_value(&mut hotkey.key_code, KeyCode::Digit7, KeyCode::Digit7.name());
            ui.radio_value(&mut hotkey.key_code, KeyCode::Digit8, KeyCode::Digit8.name());
            ui.radio_value(&mut hotkey.key_code, KeyCode::Digit9, KeyCode::Digit9.name());
            ui.checkbox(&mut modifiers.alt, "alt");
            ui.checkbox(&mut modifiers.ctrl, "ctrl");
            ui.checkbox(&mut modifiers.shift, "shift");
            ui.checkbox(&mut modifiers.meta, "meta");
            if ui.button("set").clicked() {
                if modifiers.alt {
                    hotkey.modifiers.insert(livesplit_hotkey::Modifiers::ALT);
                }
                if modifiers.ctrl {
                    hotkey.modifiers.insert(livesplit_hotkey::Modifiers::CONTROL);
                }
                if modifiers.meta {
                    hotkey.modifiers.insert(livesplit_hotkey::Modifiers::META);
                }
                if modifiers.shift {
                    hotkey.modifiers.insert(livesplit_hotkey::Modifiers::SHIFT);
                }
                hook.register(hotkey, move || {
                    println!("got HotKey {}", hotkey);
                }).expect("TODO: panic message");
            }
        });
    })
}