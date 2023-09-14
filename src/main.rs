mod gui;
mod image_coding;
mod hotkeys;
mod itc;
mod screenshot;
mod head_thread;

const DEBUG: bool = true;
fn main()
{
    gui::launch_gui();
}

