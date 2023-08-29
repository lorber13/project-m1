

use eframe::egui;
use super::super::*;
use super::rect_selection::RectSelection;
use screenshots::Screen;
extern crate image;
use super::EnumGuiState;
use std::rc::Rc;
use std::cell::RefCell;





pub struct MainWindow {
    output_format: image_coding::ImageFormat,
    area: image_coding::ScreenshotDim,
    bool_clipboard: bool,
    global_gui_state: Rc<RefCell<EnumGuiState>>
}
impl MainWindow{
    pub fn new(global_gui_state: Rc<RefCell<EnumGuiState>>) -> Self{
        Self { output_format: image_coding::ImageFormat::Png,
        area: image_coding::ScreenshotDim::Fullscreen, bool_clipboard: false,
        global_gui_state}
    }
}


 impl eframe::App for MainWindow{

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        _frame.set_fullscreen(false);
        _frame.set_maximized(false);
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
                        ui.selectable_value(&mut self.area, image_coding::ScreenshotDim::Fullscreen, "Full Screen");
                        ui.selectable_value(&mut self.area, image_coding::ScreenshotDim::Rectangle, "Rectangle");
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
                    //se l'utente ha selezionato screenshot di un'area, si fa partire il processo per la selezione dell'area
                    if self.area == image_coding::ScreenshotDim::Rectangle
                    {
                        _frame.set_visible(false);
                        let rs = RectSelection::new(self.global_gui_state.clone());
                        self.global_gui_state.replace(EnumGuiState::ShowingRectSelection(Rc::new(RefCell::new(rs))));
                    }


                    //invio, tramite Channel, di un segnale al thread principale per richiedere il salvataggio dello screenshot               
               
                }
            });
            
    }
}
  
