

use eframe::egui;
use super::{super::*, GlobalGuiState};
use screenshots::Screen;
extern crate image;
use super::super::itc::ScreenshotDim;
use std::sync::{Arc, Mutex};
use std::io::stderr;
use std::io::Write;
use std::sync::mpsc::Sender;




pub struct MainWindow {
    output_format: image_coding::ImageFormat,
    area: ScreenshotDim,
    bool_clipboard: bool,
    global_gui_state: Arc<GlobalGuiState>,
    head_thread_tx: Arc<Mutex<Sender<crate::itc::SignalToHeadThread>>>
}
impl MainWindow{
    pub fn new(global_gui_state: Arc<GlobalGuiState>, head_thread_tx: Arc<Mutex<Sender<crate::itc::SignalToHeadThread>>>) -> Self{
        Self { output_format: image_coding::ImageFormat::Png,
        area: ScreenshotDim::Fullscreen, bool_clipboard: false,
        global_gui_state,
        head_thread_tx}
    }

    fn send_acquire_signal(&self, sd: ScreenshotDim)
    {
        match self.head_thread_tx.lock().unwrap().send(crate::itc::SignalToHeadThread::AcquirePressed(sd))
        {
            Ok(_) =>(),
            Err(e) =>
            {
                if DEBUG {println!("DEBUG: attempting to lock global_gui_state {:?}", self.global_gui_state);}
                { 
                    self.global_gui_state.show_error_alert("Impossible to acquire.\nService not available.\nPlease restart the program.");
                }
                writeln!(stderr(), "{}", e);  
            }
        }
    }

}


 impl eframe::App for MainWindow{

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        _frame.set_decorations(true);
        _frame.set_fullscreen(false);
        _frame.set_maximized(false);
        _frame.set_window_size(egui::Vec2::new(500.0, 300.0));
        _frame.set_visible(true);
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
                        ui.selectable_value(&mut self.area, ScreenshotDim::Fullscreen, "Full Screen");
                        ui.selectable_value(&mut self.area, ScreenshotDim::Rectangle, "Rectangle");
                    });
                    ui.end_row();
                    ui.separator();

                egui::ComboBox::from_label("Choose the format desired:") //menù a tendina per la scelta del formato di output
                    .selected_text(format!("{:?}", self.output_format ))
                    .show_ui(ui, |ui|{
                        ui.style_mut().wrap = Some(false);
                        ui.set_min_width(60.0);
                        ui.selectable_value(&mut self.output_format, image_coding::ImageFormat::Png, "Png");
                        ui.selectable_value(&mut self.output_format, image_coding::ImageFormat::JPEG, "JPEG");
                        ui.selectable_value(&mut self.output_format, image_coding::ImageFormat::GIF, "GIF");
                    });
                    ui.end_row();
                ui.separator();

                ui.checkbox(&mut self.bool_clipboard, "Copy To Clipboard");
                
                // gestione della pressione del pulsante "Acquire"
                if ui.button("Acquire").clicked(){
                    //invio, tramite Channel, di un segnale al thread principale per richiedere il salvataggio dello screenshot
                    //se l'utente ha selezionato screenshot di un'area, si fa partire il processo per la selezione dell'area 
                    self.send_acquire_signal(self.area.clone());
               
               
                }
            });
            
    }
}
  
