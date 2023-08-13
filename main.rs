#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui::{self};
use std::fs::write;
use screenshots::Screen;
use std::fs::*;
extern crate image;
use arboard::{Clipboard, ImageData};





#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum Enum { //Enum per selezione del formato
    Png,
    JPG,
    GIF,
}
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum Enum2{ //prova per la scelta del tipo di screenshot
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

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct Content {
    output_format: Enum,
    area: Enum2,
}
impl Default for Content{
    fn default() -> Self{
        Self { output_format: Enum::Png,
        area: Enum2::Fullscreen, }
    }
}

 impl eframe::App for Content{

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let screens= Screen::all().expect("problema con gli screen");
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
                    ui.selectable_value(&mut self.output_format, Enum::JPG, "JPG");
                    ui.selectable_value(&mut self.output_format, Enum::GIF, "GIF");
                });
                ui.end_row();
            ui.separator();
            // acquisizione dello screenshot in png
            if ui.button("Acquisici").clicked(){
                for screen in screens.iter(){
                    let img: screenshots::Image=screen.capture().expect("problema con l'acquisizione");
                    let buffer=img.to_png(None).expect("problema con la conversione");
                    write("screenshot.png", buffer.clone()).expect("problema con il salvataggio");
                    
                    let mut ctx = Clipboard::new().unwrap(); //inizializzazione della clipboard per copiare negli appunti
                    let image = image::open("screenshot.png").expect("problemi");
                    let bytes = image.as_bytes();
                    
                   let img_data = ImageData {width: image.width() as usize, height: image.height() as usize, bytes: std::borrow::Cow::Borrowed(bytes)};
                   ctx.set_image(img_data).expect("no show on clipboard"); //settare l'immagine come elemento copiato negli appunti                  

                    
                   match self.output_format {
                    Enum::Png => write("screenshot.png", buffer.clone()).expect("problema con il salvataggio"),
                    Enum::JPG => conversion_into_jpg(),
                    Enum::GIF => conversion_into_gif(buffer),
                }                
             
                
                }
            }
            
        });
    }
  
}

pub fn conversion_into_jpg (){ //conversione in jpg passando per png (acquizione iniziale)
    let image_path="screenshot.png";
    let img2= image::open(image_path).unwrap();
    img2.save_with_format("screenshot.jpg", image::ImageFormat::Jpeg).expect("impossibile convertire in jpg");
}

pub fn conversion_into_gif (mut buffer:Vec<u8>){ //gif non funziona ancora
    let image_path="screenshot.png";
    let img2=image::open(image_path).unwrap();
    let frame = gif::Frame::from_rgb(img2.width() as u16,img2.height() as u16,&mut *buffer);
    let image = File::create("screenshot.gif").unwrap();
    let mut encoder = gif::Encoder::new(& image, frame.width, frame.height, &[]).unwrap();
    encoder.write_frame(&frame).unwrap();
}
