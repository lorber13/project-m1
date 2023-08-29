mod image_coding;

use eframe::{
    egui::{CentralPanel, Context},
    App, Frame,
  };
  use std::path::PathBuf;
  use rfd::FileDialog;
  
  #[derive(Default)]
  pub struct Demo {
  }
  
  impl App for Demo {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
      show_file_dialog(ctx, _frame);
    }
  }

  pub fn show_file_dialog(ctx: &Context, _frame: &mut Frame) -> Option<PathBuf>
  {
    let mut file = None;
    CentralPanel::default().show(ctx, |ui| {

        file = FileDialog::new()
            .add_filter("image", &image_coding::available_formats())
            .set_directory("/")
            .save_file();
      });

      return file;
  }
  
  fn main() {
    eframe::run_native(
      "File Dialog Demo",
      eframe::NativeOptions::default(),
      Box::new(|_cc| Box::new(Demo::default())),
    );
  }
