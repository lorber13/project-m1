
use eframe::epaint::Rect;
use egui_extras::RetainedImage;
use image::RgbaImage;

use crate::{screenshot, image_coding};
use crate::{itc::ScreenshotDim, gui::GlobalGuiState};

use super::itc::SignalToHeadThread;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

struct HeadThread 
{
    //NOTA: nel caso si volesse permettere l'esecuzione di piu' screenshot in successione molto ravvicinata, conviene trasformare i campi seguenti in code
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
                self.gui.switch_to_none();
                self.gui.switch_to_rect_selection();
            },
            ScreenshotDim::Fullscreen => self.do_fullscreen_screenshot() //TO DO: usare il codice della libreria screenshots
        }
    }

    fn do_rect_screenshot(&mut self, rect: Rect)
    {
        self.gui.switch_to_none();

        //codice del crate screenshots
        // self.screenshot = ...
    }

    fn do_fullscreen_screenshot(&mut self)
    {
        match screenshot::fullscreen_screenshot()
        {
            Err(s) => self.gui.show_error_alert(s),
            Ok(ri) => { self.screenshot = Some(ri); self.gui.show_file_dialog(); }
        }
    }

    fn manage_save_request(&mut self, pb: Option<PathBuf>)
    {
        match self.screenshot.take()
        {
            None => {   //è stato, per qualche motivo, raggiunto uno stato illegale
                write!(std::io::stderr(), "head thread received a path to save an image with head_thread.screenshot == None");
                self.gui.show_error_alert("An error occoured. Please redo the screenshot again");
            },
            Some(img) =>
            {
                match pb
                {
                    None => {   //se l'utente ha annullato l'operazione dal file dialog, si scarta l'immagine salvata
                        self.gui.switch_to_main_window()
                    },
                    Some(p) =>  
                    {
                        if let Err(s) = image_coding::save_image(p.as_path(), img)
                        {
                            write!(std::io::stderr(), "Error in saving the image: {:?}", s);
                            self.gui.show_error_alert("An error occoured. Impossible to save the image");
                        }
                    }
                }
            }
        }
        
    }
}

