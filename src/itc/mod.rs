/*Definizione di enum usate nelle interfacce di comunicazione tra diversi moduli.*/

use std::{env, time::Duration};

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ScreenshotDim {
    //Enum per la scelta del tipo di screenshot
    Fullscreen,
    Rectangle,
}

impl Clone for ScreenshotDim {
    fn clone(&self) -> Self {
        match self {
            ScreenshotDim::Fullscreen => ScreenshotDim::Fullscreen,
            ScreenshotDim::Rectangle => ScreenshotDim::Rectangle,
        }
    }
}

pub enum SettingsEvent {
    Saved,
    Aborted,
    Nil,
    OpenDirectoryDialog,
}

#[derive(Clone, Copy)]
pub struct Delay {
    pub delayed: bool,
    pub scalar: f64,
}

///Secondi
const DELAY_ANIMATIONS_WINDOWS: f32 = 0.25;
///Secondi
const DELAY_ANIMATIONS_LINUX: f32 = 0.25; 

///Ritorna la durata dell'animazione di scomparsa della finestra nello specifico
/// sistema operativo in uso.
/// Se il sistema non è tra quelli per cui l'applicazione è stata testata, ritorna
/// un delay ampio.  
pub fn get_animations_delay() -> Duration {
    match env::consts::OS {
        "windows" => Duration::from_secs_f32(DELAY_ANIMATIONS_WINDOWS),
        "linux" => Duration::from_secs_f32(DELAY_ANIMATIONS_LINUX),
        _ => Duration::from_secs(1),
    }
}
