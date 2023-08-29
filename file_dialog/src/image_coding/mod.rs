use image::{RgbaImage, ImageEncoder};
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

impl Into<&str> for ImageFormat
{
    fn into(self) -> &'static str
    {
        match self
        {
            Self::Png => "Png",
            Self::JPEG => "Jpeg",
            Self::GIF => "Gif"
        }
    }
}

pub fn available_formats() -> Vec<&'static str>
{
    vec![ImageFormat::Png.into(), ImageFormat::JPEG.into(), ImageFormat::GIF.into()]
} 

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ScreenshotDim{ //Enum per la scelta del tipo di screenshot
    Fullscreen,
    Rectangle,
}

pub fn copy_to_clipboard(img: &RgbaImage) -> Result<(), arboard::Error>
{
    let mut ctx2 = Clipboard::new().unwrap(); //inizializzazione della clipboard per copiare negli appunti
    let img_data = ImageData {width: img.width() as usize, height: img.height() as usize, bytes: std::borrow::Cow::Borrowed(img)};
    ctx2.set_image(img_data) //settare l'immagine come elemento copiato negli appunti  
}



pub fn save_in_png(file_output: File, img: RgbaImage) -> image::ImageResult<()>
{
    let pnge = image::codecs::png::PngEncoder::new(file_output);
    return pnge.write_image(&img, img.width(), img.height(), image::ColorType::Rgba8);  
}

pub fn save_in_jpeg (file_output: File, img: RgbaImage) -> image::ImageResult<()>
{   //salvataggio in jpeg senza passare da png, usando Encoder fornito dal crate image
    let mut encoder = JpegEncoder::new(file_output);
    encoder.encode(&img, img.width(), img.height(), image::ColorType::Rgba8)
}

pub fn save_in_gif (file_output: File, img: RgbaImage) -> image::ImageResult<()>
{
    let mut encoder = GifEncoder::new(file_output);  
    encoder.encode(&img, img.width() , img.height(), image::ColorType::Rgba8)
}