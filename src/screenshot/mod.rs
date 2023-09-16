use image::{RgbaImage, imageops::FilterType};
use screenshots::{Screen, DisplayInfo};
use std::io::Write;


pub fn fullscreen_screenshot(screen_id: u32) -> Result<RgbaImage, &'static str>
{
    match Screen::all().unwrap().into_iter().find(|s| s.display_info.id == screen_id)
    {
        None => return Err("invalid screen id"),
        Some(screen) => 
        {
            if crate::DEBUG {println!("DEBUG: performing fullscreen screenshot");}
            match screen.capture() // todo: modify in case of multiple monitors
            {
                Ok(shot) => return Ok(shot),
                Err(s) => { write!(std::io::stderr(), "Error: unable to perform screenshot: {:?}", s); return Err("Error: unable to perform screenshot"); }
            }
        }
    }
    
}

pub fn get_all_screens_incons(icon_width: u32) -> Vec<(Screen, RgbaImage)>
{
    Screen::all().unwrap().into_iter().map(|s|
        {
            let img = s.capture().unwrap();
            let height = icon_width*img.height() / img.width();
            (s, image::imageops::resize(&s.capture().unwrap(), icon_width, height, FilterType::Gaussian))
        }
    ).collect()
}

pub fn get_main_screen_id() -> u32
{
    Screen::all().unwrap().into_iter().find(|s|s.display_info.is_primary).unwrap().display_info.id
}