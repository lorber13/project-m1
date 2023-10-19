
use arboard::{Clipboard, ImageData};
use image::{RgbaImage, ImageError};
use std::{sync::mpsc::{Receiver, channel}, io::stdout, path::PathBuf, ffi::OsStr};
use std::fs::File;
use std::io::Write;
use eframe::emath::Rect;

use crate::DEBUG;

#[derive(Debug, PartialEq, Clone, Copy)]
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

impl ImageFormat
{
    pub fn available_formats() -> Vec<&'static str>
    {
        vec![ImageFormat::Png.into(), ImageFormat::JPEG.into(), ImageFormat::GIF.into()]
    } 
}

pub fn start_thread_copy_to_clipboard(img: &RgbaImage) -> Receiver<Result<(), arboard::Error>>
{
    let (tx, rx ) = channel();
    let i = img.clone();
    std::thread::spawn(move || 
    {
        let _ =tx.send(copy_to_clipboard(&i));
    });
    rx
}

fn copy_to_clipboard(img: &RgbaImage) -> Result<(), arboard::Error>
{
    let mut ctx2 = Clipboard::new().unwrap(); //inizializzazione della clipboard per copiare negli appunti
    let img_data = ImageData {width: img.width() as usize, height: img.height() as usize, bytes: std::borrow::Cow::Borrowed(img)};
    ctx2.set_image(img_data) //settare l'immagine come elemento copiato negli appunti  
}

pub fn start_thread_crop_image(rect: Rect, img: RgbaImage ) -> Receiver<Result<RgbaImage, &'static str>>
{
    let (tx, rx) = channel();
    std::thread::spawn(move||
    {
        let _ = tx.send(Ok(crop_image(rect, img)));
    });

    rx
}

fn crop_image(rect: Rect, img: RgbaImage) -> RgbaImage
{
    image::imageops::crop_imm::<RgbaImage>(&img,
        rect.left() as u32,
        rect.top() as u32,
        rect.width() as u32,
        rect.height() as u32).to_image()
}


pub fn start_thread_save_image(dir_path: std::path::PathBuf, file_name: String, extension: String, img: RgbaImage) -> Receiver<image::ImageResult<()>>
{
    let (tx, rx ) = channel();
    std::thread::spawn(move || 
    {
        let mut file_output = dir_path;
        if file_output.is_dir()
        {
            let mut temp: Vec<std::path::Component> = file_output.components().collect();
            temp.push(std::path::Component::Normal(OsStr::new("to_be_replaced")));
            file_output = PathBuf::from_iter(temp.iter());
        }
        file_output.set_file_name(file_name);
        file_output.set_extension(extension);

        if DEBUG {writeln!(stdout(), "DEBUG: saving new image: {}", file_output.display());}

        let _= tx.send(save_image(file_output, img));
    });
    rx
}


fn save_image(file_output: std::path::PathBuf, img: RgbaImage) -> image::ImageResult<()>
{
    if let Some(ext) = file_output.extension()
    {
        if ImageFormat::available_formats().contains(&ext.to_str().unwrap())
        {
            return match ext.to_str().unwrap() {
                "Gif" => {
                    let file = File::create(file_output).unwrap();
                    let mut encoder = image::codecs::gif::GifEncoder::new_with_speed(file, 30);
                    encoder.encode(&*img, img.width(), img.height(), image::ColorType::Rgba8)
                }
                _ => img.save(file_output)
            }
        }
        return Err(ImageError::IoError(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid file extension")));
    }
    
    return Err(ImageError::IoError(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Missing file extension")));
      
}

#[cfg(test)]
mod tests{
    #[test]
    fn copy_clipboard_test() {
        let img = image::ImageBuffer::new(0,0);
        let r = crate::image_coding::start_thread_copy_to_clipboard(&img);
        assert!(r.recv().is_ok());
    }

    #[test]
    fn save_test() {
        let img = image::RgbaImage::new(0,0);
        let r = crate::image_coding::start_thread_save_image("test.png".into(), "screenshot".to_string(), "png".to_string(), img);
        assert!(r.recv().is_ok());
    }
}
