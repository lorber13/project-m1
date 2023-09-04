
use eframe::epaint::Rect;
use image::RgbaImage;

use crate::screenshot;
use crate::{itc::ScreenshotDim, gui::GlobalGuiState};

use super::itc::SignalToHeadThread;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

struct HeadThread 
{
    gui : Arc<GlobalGuiState>,
    rx: Receiver<SignalToHeadThread>,
    //state
    screenshot: Option<RgbaImage>,
    rect: Option<Rect>
}

pub fn start_head_thread(rx: Receiver<SignalToHeadThread>, gui : Arc<GlobalGuiState>)
{
    let mut head_thr = HeadThread::new(rx, gui);
    head_thr.do_loop() 
}


impl HeadThread
{
    fn new(rx: Receiver<SignalToHeadThread>, gui: Arc<GlobalGuiState> ) -> Self
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
                    SignalToHeadThread::RectSelected(r) => self.do_rect_screenshot(r), 
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
            ScreenshotDim::Rectangle => 
            {
                self.gui.clone().switch_to_none();
                self.gui.clone().switch_to_rect_selection();
            },
            ScreenshotDim::Fullscreen => () //TO DO: usare il codice della libreria screenshots
        }
    }

    fn do_rect_screenshot(&mut self, rect: Rect)
    {
        self.gui.switch_to_none();

        //codice del crate screenshots
        // self.screenshot = ...
    }

    fn manage_save_request(&mut self, pb: PathBuf)
    {
        let pb = super::gui::file_dialog::show_file_dialog();
        
    }
}

