#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use image::codecs::{jpeg::JpegEncoder, gif::GifEncoder};
use std::fs::write;
use screenshots::Screen;
use std::fs::*;
extern crate image;
use arboard::{Clipboard, ImageData};




#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum Enum { //Enum per selezione del formato
    Png,
    JPEG,
    GIF,
}
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum Enum2{ //Enum per la scelta del tipo di screenshot
    Fullscreen,
    Rectangle,
}

fn main() {  
    let options = eframe::NativeOptions::default();
    
    eframe::run_native(
        "Simple screenshot App", 
        options,  
        Box::new(|_cc| Box::<Content>::default())
    ).unwrap();
}

struct Content {
    output_format: Enum,
    area: Enum2,
    bool_clipboard: bool
}
impl Default for Content{
    fn default() -> Self{
        Self { output_format: Enum::Png,
        area: Enum2::Fullscreen, bool_clipboard: false}
    }
}


 impl eframe::App for Content{

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let screens= Screen::all().expect("Mismatching type in Vec<Screen>");
           egui::CentralPanel::default().show(ctx, |ui|{
            ui.label("Capture Mode");
            ui.separator();
            egui::ComboBox::from_label("Area") //prova di menù a tendina per scegliere se fare uno screen di tutto, oppure per selezionare un rettangolo
                  .selected_text(format!("{:?}", self.area))
                  .show_ui(ui, |ui|{
                    ui.style_mut().wrap = Some(false);
                    ui.set_min_width(60.0);
                    ui.selectable_value(&mut self.area, Enum2::Fullscreen, "Full Screen");
                    ui.selectable_value(&mut self.area, Enum2::Rectangle, "Rectangle");
                  });
                  ui.end_row();
                  ui.separator();
            egui::ComboBox::from_label("Choose the format desired:") //menù a tendina per la scelta del formato di output
                .selected_text(format!("{:?}", self.output_format ))
                .show_ui(ui, |ui|{
                    ui.style_mut().wrap = Some(false);
                    ui.set_min_width(60.0);
                    ui.selectable_value(&mut self.output_format, Enum::Png, "Png");
                    ui.selectable_value(&mut self.output_format, Enum::JPEG, "JPEG");
                    ui.selectable_value(&mut self.output_format, Enum::GIF, "GIF");
                });
                ui.end_row();
            ui.separator();
            let checkbox_clipboard = ui.checkbox(&mut self.bool_clipboard, "Copy To Clipboard");
            
            // gestione della pressione del pulsante "Acquire"
            if ui.button("Acquire").clicked(){
                for screen in screens.iter(){
                    let img=screen.capture().expect("Problem with the acquisition of the screenshot image"); //acquisizione dello screenshot con formato screenshot::Image
                    
                    if self.bool_clipboard    //solo se la checkbox è stata selezionata, l'immagine viene copiata negli appunti 
                    {
                        let mut ctx2 = Clipboard::new().unwrap(); //inizializzazione della clipboard per copiare negli appunti
                        let bytes = img.rgba();
                        let img_data = ImageData {width: img.width() as usize, height: img.height() as usize, bytes: std::borrow::Cow::Borrowed(bytes)};
                        ctx2.set_image(img_data).expect("no show on clipboard"); //settare l'immagine come elemento copiato negli appunti  
                    }
                    
                   match self.output_format {
                    Enum::Png => save_in_png(img),
                    Enum::JPEG => save_in_jpeg(img),
                    Enum::GIF => save_in_gif(img),
                    }                
               
                }
            }
            
        });
    }
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
    let mut encoder = GifEncoder::new_with_speed(file_output, 30);
   encoder.encode(img.rgba(), img.width() , img.height(), image::ColorType::Rgba8).expect("Problem with the GIF encoder");
}
