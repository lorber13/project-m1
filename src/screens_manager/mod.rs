use image::{RgbaImage, imageops::FilterType};
use screenshots::{Screen, DisplayInfo};
use std::io::Write;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Mutex, Arc};

pub struct ScreensManager
{
    pub screens: Vec<(Screen, Arc<Mutex<Option<RgbaImage>>>)>,
    pub curr_screen_index: usize,
    icon_width: u32
}

impl ScreensManager
{
    pub fn new(icon_width: u32) -> Self
    {
        let screens = screenshots::Screen::all().unwrap();
        let mut ret = Self {screens: Self::load_icons(screens, icon_width),curr_screen_index: 0, icon_width};
        ret.select_primary_screen();
        ret
    }


    ///Aggiorna il vettore di Screen, rilevando le modifiche hardware.
    /// Anche l'indice viene modificato, nel caso lo schermo precedentemente selezionato cambi
    /// di posizione nel vettore.
    /// Nel caso lo schermo precedentemente selezionato non venga piu' rilevato,
    /// di default viene selezionato quello primario
    pub fn update_available_screens(&mut self)
    {
        let curr_id = self.screens.get::<usize>(self.curr_screen_index).unwrap().0.display_info.id;
        self.screens = Self::load_icons(Screen::all().unwrap(), self.icon_width);
        match self.screens.iter().position(|s|s.0.display_info.id == curr_id)
        {
            Some(i) => self.curr_screen_index = i,
            None => self.select_primary_screen()
        }
    }

    pub fn select_screen(&mut self, index: usize)
    {
        self.curr_screen_index = index;
    }
    
    pub fn select_primary_screen(&mut self)
    {
        self.curr_screen_index = self.screens.iter().position(|s|s.0.display_info.is_primary).unwrap();
    }

    pub fn start_thread_fullscreen_screenshot(&self) -> Receiver<Result<RgbaImage, &'static str>>
    {
        let (tx, rx) = channel();
        let sc = self.clone();
        std::thread::spawn(move||
            {
                tx.send(sc.fullscreen_screenshot()).expect("thread performing fullscreen screenshot was not able to send throught the channel");
            });
        rx
    }

    fn fullscreen_screenshot(&self) -> Result<RgbaImage, &'static str>
    {
        if crate::DEBUG {println!("DEBUG: performing fullscreen screenshot");}
        
        match self.screens.get(self.curr_screen_index).unwrap().0.capture() 
        {
            Ok(shot) => return Ok(shot),
            Err(s) => { write!(std::io::stderr(), "Error: unable to perform screenshot: {:?}", s); return Err("Error: unable to perform screenshot"); }
        }
        
    }

    pub fn get_current_screen_infos(&self) ->DisplayInfo
    {
        self.screens.get(self.curr_screen_index).unwrap().0.display_info
    }

    pub fn get_current_screen_icon(&self) -> Arc<Mutex<Option<RgbaImage>>>
    {
        self.screens.get(self.curr_screen_index).unwrap().1.clone()
    }

    fn load_icons(v: Vec<Screen>, icon_width: u32) -> Vec<(Screen, Arc<Mutex<Option<RgbaImage>>>)>
    {
        let mut ret = vec![];
        for s in v.into_iter()
        {
            let arc = Arc::new(Mutex::new(None));
            ret.push((s, arc.clone()));
            std::thread::spawn(move||
            {
                let img = s.capture().unwrap();
                let height = icon_width*img.height() / img.width();
                let icon = image::imageops::resize(&s.capture().unwrap(), icon_width, height, FilterType::Gaussian);
                let mut g = arc.lock().unwrap();
                *g = Some(icon);
            });
        }
        ret
    }
}

impl Clone for ScreensManager
{
    fn clone(&self) -> Self {
        Self {screens: self.screens.clone(), curr_screen_index: self.curr_screen_index, icon_width: self.icon_width}
    }
}

