use screenshots::Image;
use arboard::{Clipboard, ImageData};
use image::codecs::{jpeg::JpegEncoder, gif::GifEncoder};
use std::fs::*;

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ImageFormat { //Enum per selezione del formato
    Png,
    JPEG,
    GIF,
}
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ScreenshotDim{ //Enum per la scelta del tipo di screenshot
    Fullscreen,
    Rectangle,
}

pub fn copy_to_clipboard(img: &Image)
{
    let mut ctx2 = Clipboard::new().unwrap(); //inizializzazione della clipboard per copiare negli appunti
    let bytes = img.rgba();
    let img_data = ImageData {width: img.width() as usize, height: img.height() as usize, bytes: std::borrow::Cow::Borrowed(bytes)};
    ctx2.set_image(img_data).expect("no show on clipboard"); //settare l'immagine come elemento copiato negli appunti  
}

pub fn save_in_png(img: screenshots::Image){
    let buffer=img.to_png(None).expect("Problem with the conversion in buffer");
    write("screenshot.png", buffer).expect("Problem with the file saving");
}
pub fn save_in_jpeg (img: screenshots::Image){   //salvataggio in jpeg senza passare da png, usando Encoder fornito dal crate image
    let file_output = File::create("screenshot.jpeg").expect("Problem with the creation of file JPEG");
    let mut encoder = JpegEncoder::new(file_output);
    encoder.encode(img.rgba(), img.width(), img.height(), image::ColorType::Rgba8).expect("Problem with the JPEG encoder"); 
}

pub fn save_in_gif (img: screenshots::Image){
    let file_output = File::create("screenshot.gif").expect("Problem with the creation of file gif"); 
    let mut encoder = GifEncoder::new(file_output);  
   encoder.encode(img.rgba(), img.width() , img.height(), image::ColorType::Rgba8).expect("Problem with the GIF encoder");
}