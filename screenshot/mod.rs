use image::RgbaImage;
use screenshots::Screen;
use std::io::Write;


pub fn fullscreen_screenshot() -> Result<RgbaImage, &'static str>
{
    if crate::DEBUG {println!("DEBUG: performing fullscreen screenshot");}
    match Screen::all().unwrap().first().unwrap().capture() // todo: modify in case of multiple monitors
    {
        Ok(shot) => Ok(shot),
        Err(s) => { write!(std::io::stderr(), "Error: unable to perform screenshot: {:?}", s); return Err("Error: unable to perform screenshot"); }
    }
    
}

pub fn rect_screenshot() // -> Result<RetainedImage, &'static str>
{
    //TO DO
}