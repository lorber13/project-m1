
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, hotkey};
use global_hotkey::hotkey::HotKey;
use std::cmp::Ordering;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::mpsc::{Receiver, channel, Sender};
use std::sync::{Arc, RwLock};

use crate::DEBUG;

pub const N_HOTK: usize = 2;        //il numero di hotkey diverse presenti nella enum sottostante
#[derive(Clone, Copy)]
pub enum HotkeyName
{
    FullscreenScreenshot,
    RectScreenshot
}

impl PartialEq for HotkeyName
{
    fn eq(&self, other: &Self) -> bool {
        <HotkeyName as Into<usize>>::into(*self) == <HotkeyName as Into<usize>>::into(*other)
    }
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
    backup: Vec<RwLock<Option<(HotKey, String)>>>,
    pub vec: Vec<RwLock<Option<String>>>, 
    ghm: GlobalHotKeyManager,
    listen_enabled: RwLock<bool>
}



impl RegisteredHotkeys
{
    pub fn new() -> Arc<Self>
    {
        let mut vec = vec![];
        let mut backup = vec![];
        for _ in 0..N_HOTK {vec.push(RwLock::new(None)); backup.push(RwLock::new(None));}
        let ret = Self { vec, backup, ghm: GlobalHotKeyManager::new().unwrap(), listen_enabled: RwLock::new(true) };
        Arc::new(ret)
    }

    pub fn update_changes(self: &Arc<Self>) -> Result<(), String>
    {
        let mut ret = Ok(());
        for i in 0..N_HOTK
        {
            if DEBUG {println!("DEBUG: comparing hotkeys {}", i);}

            let temp1 ;
                let temp2;
                {
                    temp1 = self.vec.get(i).unwrap().read().unwrap().clone();
                    temp2 = self.backup.get(i).unwrap().read().unwrap().clone();
                }
            match (temp1, temp2)
            {
                (None, None) => (),
                (None, Some(..)) => ret = self.unregister(HotkeyName::from(i)),
                (Some(s), None) => ret = self.register(s.to_string(), HotkeyName::from(i)),
                (Some(s1), Some((_, s2))) =>
                {
                    if s1.cmp(&s2) != Ordering::Equal
                    {
                        ret = self.register(s1.to_string(), HotkeyName::from(i))
                    }
                }
            }
            
            if ret.is_err() {return ret;}
            if DEBUG {println!("DEBUG: hotkeys {} done", i);}
        }

        ret
    }

    pub fn prepare_for_updates(self: &Arc<Self>) -> Receiver<()>
    {
        let (tx, rx) = channel();
        let self_clone = self.clone();

        std::thread::spawn(move||
        {
            for i in 0..N_HOTK
            {
                let temp1 ;
                let temp2;
                {
                    temp1 = self_clone.vec.get(i).unwrap().read().unwrap().clone();
                    temp2 = self_clone.backup.get(i).unwrap().read().unwrap().clone();
                }
                match (temp1, temp2)
                {
                    (None, None) => (),
                    (None, Some((_, s))) => {self_clone.vec.get(i).unwrap().write().unwrap().replace(s.clone()); },
                    (Some(s), None) => {self_clone.vec.get(i).unwrap().write().unwrap().take(); },
                    (Some(s1), Some((_, s2))) =>
                    {
                        if s1.cmp(&s2) != Ordering::Equal
                        {
                            self_clone.vec.get(i).unwrap().write().unwrap().replace(s2.clone());
                        }
                    }
                }
            }
            tx.send(());
        });
        rx
    }

    fn check_if_already_registered(self: &Arc<Self>, hotkey: &String) -> bool
    {
        for opt in self.vec.iter()
        {
            if let Some( s) = &*opt.read().unwrap()
            {
                if DEBUG {println!("\nDEBUG: comparing strings {} and {}", s, hotkey);}
                if s == hotkey {return true;}
            }
        }

        false
    }

    pub fn request_register(self: &Arc<Self>, h_str: String, name: HotkeyName, tx: Sender<Result<(), &'static str>>) 
    {
        let self_clone = self.clone();

        std::thread::spawn(move||
        {
            let mut ret = Ok(());

            //controllo che la stessa combinazione di tasti non sia già associata ad un altro comando:
            if self_clone.check_if_already_registered(&h_str) {ret= Err("Hotkey already registered");}
            else {
                if let Ok(h) = HotKey::from_str(&h_str)
                {
                    self_clone.vec.get(<HotkeyName as Into<usize>>::into(name)).unwrap().write().unwrap().replace(h_str);
                }
            }

            tx.send(ret);
        });
    }


    /// NON è possibile fare eseguire da un thread separato perchè la libreria GlobalHotkey non funziona
    fn register(self: &Arc<Self>, h_str: String, name: HotkeyName) -> Result<(), String>
    {
        if let Ok(h) = HotKey::from_str(&h_str)
        {
            //if crate::DEBUG {println!("\nDEBUG: Hotkey not registered yet");}

            match self.ghm.register(h) 
            { 
                Ok(()) =>
                {
                    if DEBUG{println!("DEBUG: hotkey registered.\n The lock is {:?}", self.backup.get(<HotkeyName as Into<usize>>::into(name)).unwrap());}
                    self.backup.get(<HotkeyName as Into<usize>>::into(name)).unwrap().write().unwrap().replace((h, h_str));
                    if DEBUG{println!("DEBUG: backup modified");}
                    return Ok(());
                },
                Err (e) => return Err(format!("Unable to register the hotkey related to command {}.\nError: {}", <HotkeyName as Into<String>>::into(name), e.to_string()))
                
            } 
            
        }
        
        return Err(format!("Unable to register the hotkey related to command {}", <HotkeyName as Into<String>>::into(name)));
    }

    pub fn request_unregister(self: &Arc<Self>, name: HotkeyName)
    {
        let _ = self.vec.get(<HotkeyName as Into<usize>>::into(name)).unwrap().write().unwrap().take();
    }

    /// NON è possibile fare eseguire da un thread separato perchè la libreria GlobalHotkey non funziona
    fn unregister(self: &Arc<Self>, name: HotkeyName) -> Result<(), String>
    {
        let temp = self.backup.get(<HotkeyName as Into<usize>>::into(name)).unwrap().write().unwrap().take();
        if let Some((h, s)) = temp 
        {
            if self.ghm.unregister(h).is_ok()
            {
                return Ok(());
            }
        }
        return Err(format!("Unable to unregister the hotkey related to command {}", <HotkeyName as Into<String>>::into(name)));
    }

    pub fn listen_hotkeys(self: &Arc<Self>) -> Option<HotkeyName>
    {
        if ! *self.listen_enabled.read().unwrap() {return None;}

        if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv()
        {
            for (i, opt) in self.backup.iter().enumerate()
            {
                match opt.read().unwrap().clone()
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

    pub fn get_hotkey_string(self: &Arc<Self>, name: HotkeyName) -> Option<String>
    {
        match self.vec.get(<HotkeyName as Into<usize>>::into(name)).unwrap().read().unwrap().as_deref()
        {
            None => None,
            Some(hk_str) => Some(hk_str.to_string())
        }
        
    }

    pub fn set_listen_enabled(&self, val: bool)
    {
        *self.listen_enabled.write().unwrap() = val;
    }
}
