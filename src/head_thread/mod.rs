
use eframe::epaint::Rect;
use image::RgbaImage;

use crate::{itc::ScreenshotDim, gui::GlobalGuiState};

use super::itc::SignalToHeadThread;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

struct HeadThread 
{
    gui : Arc<Mutex<GlobalGuiState>>,
    rx: Receiver<SignalToHeadThread>,
    //state
    screenshot: Option<RgbaImage>,
    rect: Option<Rect>
}

pub fn start_head_thread(rx: Receiver<SignalToHeadThread>, gui : Arc<Mutex<GlobalGuiState>>)
{
    let mut head_thr = HeadThread::new(rx, gui);
    head_thr.do_loop() 
}


impl HeadThread
{
    fn new(rx: Receiver<SignalToHeadThread>, gui: Arc<Mutex<GlobalGuiState>> ) -> Self
    {
        Self{rx, gui, screenshot: None, rect: None}
    }

    fn do_loop(&mut self)
    {
        loop
        {
            if let Ok(sig) = self.rx.recv()
            {
                match sig
                {
                    SignalToHeadThread::Shutdown => break,
                    SignalToHeadThread::AcquirePressed(sd) => self.manage_acquire_request(sd),
                    SignalToHeadThread::RectSelected(r) => self.do_screenshot(), 
                    SignalToHeadThread::PathSelected(pb) => self.manage_save_request(pb)
                }
            }else {
                break;
            }
        }
    }

    fn manage_acquire_request(&mut self, sd: ScreenshotDim)
    {
        match sd
        {
            ScreenshotDim::Rectangle => self.gui.lock().unwrap().switch_to_rect_selection(self.gui.clone()),
            ScreenshotDim::Fullscreen => () //TO DO: usare il codice della libreria screenshots
        }
    }

    fn do_screenshot(&mut self)
    {
        //codice del crate screenshots
        // self.screenshot = ...
    }

    fn manage_save_request(&mut self, pb: PathBuf)
    {
        let pb = super::gui::file_dialog::show_file_dialog();
        
    }
}

