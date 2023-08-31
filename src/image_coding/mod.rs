
use arboard::{Clipboard, ImageData};

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

impl ImageFormat
{
    pub fn available_formats() -> Vec<&'static str>
{
    vec![ImageFormat::Png.into(), ImageFormat::JPEG.into(), ImageFormat::GIF.into()]
} 
}

pub fn copy_to_clipboard(img: &RgbaImage) -> Result<(), arboard::Error>
{
    let mut ctx2 = Clipboard::new().unwrap(); //inizializzazione della clipboard per copiare negli appunti
    let img_data = ImageData {width: img.width() as usize, height: img.height() as usize, bytes: std::borrow::Cow::Borrowed(img)};
    ctx2.set_image(img_data) //settare l'immagine come elemento copiato negli appunti  
}



pub fn save_image(file_output: &std::path::Path, img: RgbaImage) -> image::ImageResult<()>
{
    if let Some(ext) = file_output.extension()
    {
        if ImageFormat::available_formats().contains(&ext.to_str().unwrap())
        {
            return img.save(file_output);
        }
        return Err(ImageError::IoError(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid file extension")));
    }
    
    return Err(ImageError::IoError(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Missing file extension")));
      
}
