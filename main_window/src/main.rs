#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use crate::egui::Pos2;
use crate::egui::Color32;
use crate::egui::Rect;
use crate::egui::Stroke;
use crate::egui::Rounding;

const DEBUG: bool = true; //if true, it prints messages on console

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        ..Default::default()
    };

    // Stato dell'applicazione: le coordinate dei punti selezionati durante drag & drop
          

    eframe::run_simple_native("Area selection", options, move |ctx, _frame| {

        egui::Window::new("Screenshot")
        .show(ctx, |ui|
        {
            
               
        });
    })
}
