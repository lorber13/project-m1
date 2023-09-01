mod gui;
mod image_coding;
mod hotkeys;
mod itc;
use std::sync::mpsc::channel;
use std::sync::Arc;

fn main()
{
    let (tx, rx) = channel::<itc::SignalToHeadThread>();
    drop(rx);
    let arc_tx = Arc::new(tx);
    gui::launch_gui(arc_tx.clone());
}

