
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
use global_hotkey::hotkey::HotKey;
use std::str::FromStr;
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, RwLock};

pub const N_HOTK: usize = 2;        //il numero di hotkey diverse presenti nella enum sottostante
pub enum HotkeyName
{
    FullscreenScreenshot,
    RectScreenshot
}

impl Into<usize> for HotkeyName
{
    fn into(self) -> usize
    {
        match self 
        {
            Self::FullscreenScreenshot => 0,
            Self::RectScreenshot => 1
        }
    }
}

impl Into<String> for HotkeyName
{
    fn into(self) -> String
    {
        match self 
        {
            Self::FullscreenScreenshot => String::from("Fullscreen screenshot"),
            Self::RectScreenshot => String::from("Rect screenshot")
        }
    }
}

impl From<usize> for HotkeyName
{
    fn from( us: usize) -> Self
    {
        match us
        {
            0 => Self::FullscreenScreenshot,
            1 => Self::RectScreenshot,
            _ => unreachable!("Invalid value in HotkeyName::from::<usize>()")
        }
    }
}

pub struct RegisteredHotkeys
{
    pub vec: RwLock<Vec<Option<(HotKey, String)>>>,
    ghm: Arc<GlobalHotKeyManager>
}



impl RegisteredHotkeys
{
    pub fn new() -> Arc<Self>
    {
        let mut vec = vec![];
        for _ in 0..N_HOTK {vec.push(None);}
        let ret = Self { vec: RwLock::new(vec), ghm: Arc::new(GlobalHotKeyManager::new().unwrap()) };
        Arc::new(ret)
    }

    pub fn create_copy(self: &Arc<Self>) -> Receiver<Arc<Self>>
    {
        let (tx, rx) = channel();
        let clone = self.clone();

        std::thread::spawn(move||
        {
            let mut vec: Vec<Option<(HotKey, String)>> = vec![];
            for opt in clone.vec.read().unwrap().iter()
            {
                match opt
                {
                    None => vec.push(None),
                    Some((h, s)) => vec.push(Some((h.clone(), s.clone())))
                }
            }

            tx.send(Arc::new(Self {vec: RwLock::new(vec), ghm: clone.ghm.clone()}))
        });

        rx
    }


    //TO DO: fare eseguire da un thread separato
    pub fn register(self: &Arc<Self>, h_str: String, name: HotkeyName) -> Result<(), &'static str>
    {
        if let Ok(h) = HotKey::from_str(&h_str)
        {
            if self.ghm.register(h).is_ok() 
            { 
                let mut v = self.vec.write().unwrap();
                v.get_mut(<HotkeyName as Into<usize>>::into(name)).unwrap().replace((h, h_str));
                return Ok(());
            } 
            
        }
        
        return Err("Unable to register the hotkey");
    }

    //TO DO: fare eseguire da un thread separato
    pub fn unregister(self: &Arc<Self>, name: HotkeyName) -> Result<(), &'static str>
    {
        let temp = self.vec.write().unwrap().get_mut(<HotkeyName as Into<usize>>::into(name)).unwrap().take();
        if let Some((h, _)) = temp 
        {
            if self.ghm.unregister(h).is_ok()
            {
                return Ok(());
            }
        }
        return Err("Unable to unregister the hotkey ");
    }

    pub fn listen_hotkeys(self: &Arc<Self>) -> Option<HotkeyName>
    {
        if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv()
        {
            for (i, opt) in self.vec.read().unwrap().iter().enumerate()
            {
                match opt
                {
                    None => (),
                    Some((h, _)) =>
                    {
                        if h.id() == event.id
                        {
                            return Some(HotkeyName::from(i));
                        }
                    }
                    
                }
                
            }
        }

        return None;
    }

    pub fn get_string(self: &Arc<Self>, name: HotkeyName) -> Option<String>
    {
        if let Some(opt) = self.vec.read().unwrap().get(<HotkeyName as Into<usize>>::into(name))
        {
            match opt
            {
                None => None,
                Some((_, hk_str)) => Some(String::clone(hk_str))
            }
        }else {None}
        
    }
}
