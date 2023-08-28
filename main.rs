#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod output_format;

use eframe::egui;
use output_format::ScreenshotDim;
use std::fs::write;
use screenshots::Screen;
extern crate image;




fn main() {  
    let options = eframe::NativeOptions::default();
    
    eframe::run_native(
        "Simple screenshot App", 
        options,  
        Box::new(|_cc| Box::<Content>::default())
    ).unwrap();
}

struct Content {
    output_format: output_format::ImageFormat,
    area: output_format::ScreenshotDim,
    bool_clipboard: bool
}
impl Default for Content{
    fn default() -> Self{
        Self { output_format: output_format::ImageFormat::Png,
        area: output_format::ScreenshotDim::Fullscreen, bool_clipboard: false}
    }
}


 impl eframe::App for Content{

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        let screens= Screen::all().expect("Mismatching type in Vec<Screen>");

           egui::CentralPanel::default().show(ctx, |ui|
            {
                ui.label("Capture Mode");
                ui.separator();
                egui::ComboBox::from_label("Area") //prova di menù a tendina per scegliere se fare uno screen di tutto, oppure per selezionare un rettangolo
                    .selected_text(format!("{:?}", self.area))
                    .show_ui(ui, |ui|{
                        ui.style_mut().wrap = Some(false);
                        ui.set_min_width(60.0);
                        ui.selectable_value(&mut self.area, output_format::ScreenshotDim::Fullscreen, "Full Screen");
                        ui.selectable_value(&mut self.area, output_format::ScreenshotDim::Rectangle, "Rectangle");
                    });
                    ui.end_row();
                    ui.separator();

                egui::ComboBox::from_label("Choose the format desired:") //menù a tendina per la scelta del formato di output
                    .selected_text(format!("{:?}", self.output_format ))
                    .show_ui(ui, |ui|{
                        ui.style_mut().wrap = Some(false);
                        ui.set_min_width(60.0);
                        ui.selectable_value(&mut self.output_format, output_format::ImageFormat::Png, "Png");
                        ui.selectable_value(&mut self.output_format, output_format::ImageFormat::JPEG, "JPEG");
                        ui.selectable_value(&mut self.output_format, output_format::ImageFormat::GIF, "GIF");
                    });
                    ui.end_row();
                ui.separator();

                ui.checkbox(&mut self.bool_clipboard, "Copy To Clipboard");
                
                // gestione della pressione del pulsante "Acquire"
                if ui.button("Acquire").clicked(){
                    //se l'utente ha selezionato screenshot di un'area, si fa partire il processo per la selezione dell'area
                    if self.area == ScreenshotDim::Rectangle
                    {
                        let out = std::process::Command::new(".\\rect_selection\\target\\debug\\rect_selection")
                                                                    .output().unwrap();
                        let rect = serde_json::from_str(out.stdout);
                    }


                    //invio, tramite Channel, di un segnale al thread principale per richiedere il salvataggio dello screenshot               
               
                }
            }
            
        });
    }
}
  