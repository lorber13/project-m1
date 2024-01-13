/* Modulo dedicato all'elaborazione di immagini: scrittura in memoria secondaria, copia nella clipboard e ritaglio.
Siccome sono operazioni onerose, per ogni funzionalità sono messi a disposizione metodi per lanciare un thread worker. */

use arboard::{Clipboard, ImageData};
use eframe::emath::Rect;
use image::{ImageError, RgbaImage};
use std::fs::File;
use std::io::Write;
use std::{
    io::stdout,
    sync::mpsc::{channel, Receiver},
};

use crate::DEBUG;

#[derive(Debug, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ImageFormat {
    Png,
    Jpeg,
    Gif,
}

impl Into<&str> for ImageFormat {
    fn into(self) -> &'static str {
        match self {
            Self::Png => "Png",
            Self::Jpeg => "Jpeg",
            Self::Gif => "Gif",
        }
    }
}

impl From<&str> for ImageFormat {
    fn from(s: &str) -> Self {
        match s {
            "Png" | "PNG" | "png" => Self::Png ,
            "Jpeg" | "JPEG" | "jpeg" => Self::Jpeg ,
            "Gif" | "gif" | "GIF" => Self::Gif ,
            &_ => {unreachable!("Non recognized extension");}
        }
    }
}

impl ImageFormat {
    ///Utility per ottenere l'elenco dei formati contenuti nella enum sotto forma di stringhe.
    pub fn available_formats() -> Vec<ImageFormat> {
        vec![
            ImageFormat::Png,
            ImageFormat::Jpeg,
            ImageFormat::Gif,
        ]
    }
}

///Crea un canale e muove il suo  <i>Sender</i> ad un nuovo thread, il quale si occupa di eseguire <i>copy_to_clipboard()</i>
///ed inviare il risultato sul canale.
pub fn start_thread_copy_to_clipboard(img: &RgbaImage) -> Receiver<Result<(), arboard::Error>> {
    let (tx, rx) = channel();
    let i = img.clone();
    std::thread::spawn(move || {
        let _ = tx.send(copy_to_clipboard(&i));
    });
    rx
}

fn copy_to_clipboard(img: &RgbaImage) -> Result<(), arboard::Error> {
    let mut ctx2 = Clipboard::new().unwrap(); //inizializzazione della clipboard per copiare negli appunti
    let img_data = ImageData {
        width: img.width() as usize,
        height: img.height() as usize,
        bytes: std::borrow::Cow::Borrowed(img),
    };
    ctx2.set_image(img_data) //settare l'immagine come elemento copiato negli appunti
}

///Crea un canale e muove il suo  <i>Sender</i> ad un nuovo thread, il quale si occupa di eseguire <i>crop_image()</i>
///ed inviare il risultato sul canale.
///Ritorna <i>Receiver<Result<...>></i>, nonostante nell'elaborazione non possano verificarsi errori, per maggiore comprensibilità nell'uso del metodo
///e per una maggiore uniformità che rende il codice più mantenibile.
pub fn start_thread_crop_image(
    rect: Rect,
    img: RgbaImage,
) -> Receiver<Result<RgbaImage, &'static str>> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let _ = tx.send(Ok(crop_image(rect, img)));
    });

    rx
}

fn crop_image(rect: Rect, img: RgbaImage) -> RgbaImage {
    image::imageops::crop_imm::<RgbaImage>(
        &img,
        rect.left() as u32,
        rect.top() as u32,
        rect.width() as u32,
        rect.height() as u32,
    )
    .to_image()
}

///Crea un canale e muove il suo  <i>Sender</i> ad un nuovo thread, il quale si occupa di eseguire <i>save_image()</i>
///ed inviare il risultato sul canale.
pub fn start_thread_save_image(
    path: std::path::PathBuf,
    img: RgbaImage,
) -> Receiver<Result<String, ImageError>> {
    let (tx, rx) = channel();
    let path_str = path.as_os_str().to_str().unwrap().to_string();
    std::thread::spawn(move || {

        let _ = tx.send(save_image(path, img)
                        .map_or_else(|res| Err(res), |()| Ok(path_str)));
    });
    rx
}

///Controlla che l'estensione del file di output sia tra i formati supportati.
///Se il formato è GIF, esegue codice specifico per accelerare il salvataggio.
fn save_image(file_output: std::path::PathBuf, img: RgbaImage) -> image::ImageResult<()> {
    if let Some(ext) = file_output.extension() {
        if ImageFormat::available_formats().contains(&ImageFormat::from(ext.to_str().unwrap())) {
            return match ext.to_str().unwrap() {
                "Gif" => {
                    let file = File::create(file_output).unwrap();
                    let mut encoder = image::codecs::gif::GifEncoder::new_with_speed(file, 30);
                    encoder.encode(&img, img.width(), img.height(), image::ColorType::Rgba8)
                }
                _ => img.save(file_output),
            };
        }
        return Err(ImageError::IoError(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid file extension",
        )));
    }

    Err(ImageError::IoError(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "Missing file extension",
    )))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn copy_clipboard_test() {
        let img = image::ImageBuffer::new(0, 0);
        let r = crate::image_coding::start_thread_copy_to_clipboard(&img);
        assert!(r.recv().is_ok());
    }

    #[test]
    fn save_test() {
        let img = image::RgbaImage::new(0, 0);
        let r = crate::image_coding::start_thread_save_image(
            "./test.png".into(),
            img,
        );
        assert!(r.recv().is_ok());
        assert!(PathBuf::from("./test.png").exists());
    }
}
