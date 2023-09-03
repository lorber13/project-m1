mod gui;
mod image_coding;
mod hotkeys;
mod itc;
mod screenshot;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
mod head_thread;

const DEBUG: bool = true;
fn main()
{
    let (tx, rx) = channel::<itc::SignalToHeadThread>();
    let arc_tx = Arc::new(Mutex::new(tx));
    let gui = gui::new_gui(arc_tx.clone());
    let gui_clone = gui.clone();
    std::thread::spawn(move ||head_thread::start_head_thread(rx, gui_clone));
    gui::launch_gui(gui.clone());
}

