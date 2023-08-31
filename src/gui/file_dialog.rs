

use std::path::PathBuf;
use rfd::FileDialog;
  
pub fn show_file_dialog(ctx: &Context, _frame: &mut Frame) -> Option<PathBuf>
{
  let mut file = None;
  CentralPanel::default().show(ctx, |ui| {

      file = FileDialog::new()
          .add_filter("image", &super::image_coding::ImageFormat::available_formats())
          .set_directory("/")
          .save_file();
    });

  return file;
}
