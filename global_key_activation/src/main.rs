use std::thread;
use std::time::Duration;
use livesplit_hotkey::{Hook, Hotkey};
use livesplit_hotkey::KeyCode::Digit0;

fn main() {
    let hook = Hook::new().unwrap();
    hook.register(Hotkey::from(Digit0), || {
        println!("got key 0");
    }).unwrap();

    loop {
        thread::sleep(Duration::from_secs(1));
    }
}