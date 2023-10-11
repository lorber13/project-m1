
use image::{RgbaImage, imageops::FilterType};
use screenshots::{Screen, DisplayInfo};
use std::io::Write;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Mutex, Arc, RwLock, RwLockReadGuard};

pub struct ScreensManager
{
    pub screens: RwLock<Vec<(Screen, Mutex<Option<RgbaImage>>)>>, //TO DO: valutare RwLock (al posto del Mutex) anche per le icone
    curr_screen_index: RwLock<usize>,
    icon_width: u32
}

impl ScreensManager
{
    pub fn new(icon_width: u32) -> Arc<Self>
    {
        let ret = Arc::new(Self {screens: RwLock::new(vec![]),curr_screen_index: RwLock::new(0), icon_width});
        ret.update_available_screens();
        ret.select_primary_screen();
        ret
    }


    ///Aggiorna il vettore di Screen, rilevando le modifiche hardware.
    /// Anche l'indice viene modificato, nel caso lo schermo precedentemente selezionato cambi
    /// di posizione nel vettore.
    /// Nel caso lo schermo precedentemente selezionato non venga piu' rilevato,
    /// di default viene selezionato quello primario
    pub fn update_available_screens(self: &Arc<Self>) 
    {
        let arc_clone = self.clone();
        std::thread::spawn(move||
        {
            let curr_id = if !arc_clone.get_screens().is_empty()
            {
                Some(arc_clone.get_current_screen_infos().unwrap().id)
            }else {None};

            {
                let mut write_lk = arc_clone.screens.write().unwrap();
                write_lk.clear();
                for s in Screen::all().unwrap()
                {
                    write_lk.push((s, Mutex::new(None)));
                }
            }
            arc_clone.load_icons();

            if let Some(id) = curr_id
            {
                match arc_clone.get_screens().iter().position(|s|s.0.display_info.id == id)
                {
                    Some(i) => *arc_clone.curr_screen_index.write().unwrap() = i,
                    None => arc_clone.select_primary_screen()
                }
            }
            
        });
        
    }

    pub fn select_screen(self :&Arc<Self>, index: usize)
    {
        if index < self.get_screens().len()
        {
            *self.curr_screen_index.write().unwrap() = index;
        }
    }
    
    pub fn select_primary_screen(self :&Arc<Self>)
    {
        if let Some(i) = self.get_screens().iter().position(|s|s.0.display_info.is_primary)
        {
            *self.curr_screen_index.write().unwrap() = i;
        }   
    }

    pub fn start_thread_fullscreen_screenshot(self :&Arc<Self>) -> Receiver<Result<RgbaImage, &'static str>>
    {
        let (tx, rx) = channel();
        let sc = self.clone();
        std::thread::spawn(move||
            {
                tx.send(sc.fullscreen_screenshot()).expect("thread performing fullscreen screenshot was not able to send throught the channel");
            });
        rx
    }

    fn fullscreen_screenshot(self :&Arc<Self>) -> Result<RgbaImage, &'static str>
    {
        if crate::DEBUG {println!("DEBUG: performing fullscreen screenshot");}
        
        match self.get_screens().get(*self.curr_screen_index.read().unwrap()).unwrap().0.capture() 
        {
            Ok(shot) => return Ok(shot),
            Err(s) => { let _ = write!(std::io::stderr(), "Error: unable to perform screenshot: {:?}", s); return Err("Error: unable to perform screenshot"); }
        }
        
    }

    pub fn get_current_screen_index(self :&Arc<Self>) -> usize
    {
        *self.curr_screen_index.read().unwrap()
    }

    ///Ritorna None nel caso le info sugli schermi non siano ancora state caricate (vettore di screen vuoto).
    pub fn get_current_screen_infos(self :&Arc<Self>) -> Option<DisplayInfo>
    {
        match self.get_screens().get(*self.curr_screen_index.read().unwrap())
        {
            Some((screen, _)) => Some(screen.display_info),
            None => None
        }
    }

    ///Spawna un thread per ogni screen nel vettore di screen per parallelizzare la creazione di tutte le corrispondenti icone.
    /// In particolare, ogni thread scatta uno screenshot del proprio schermo, poi ridimensiona l'immagine 
    /// (riducendola alla dimensione specificata in ScreensManager::icon_width) e la salva nella corretta posizione all'interno
    /// del vettore di screen.
    fn load_icons(self: &Arc<Self>)
    {
        for (index, _) in self.get_screens().iter().enumerate()
        {
            let arc = self.clone();
            std::thread::spawn(move||
            {
                let screens = arc.get_screens();
                let (s, i) = screens.get(index).unwrap();
                let img = s.capture().unwrap();
                let height = arc.icon_width*img.height() / img.width();
                let icon = image::imageops::resize(&s.capture().unwrap(), arc.icon_width, height, FilterType::Gaussian);
                let mut g = i.lock().unwrap();
                *g = Some(icon);
            });
        }
    
    }

    pub fn get_screens<'a>(self: &'a Arc<Self>) -> RwLockReadGuard<'a, Vec<(Screen, Mutex<Option<RgbaImage>>)>>
    {
       self.screens.read().unwrap()
    }
}

#[cfg(test)]
mod tests{
    #[test]
    fn test_fullscreen() {
        let sm = crate::screens_manager::ScreensManager::new(10);
        let r = sm.start_thread_fullscreen_screenshot();
        assert!(r.recv().is_ok());
    }
}
