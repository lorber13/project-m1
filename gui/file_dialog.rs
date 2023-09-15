

use std::path::PathBuf;
use rfd::FileDialog;

  
pub fn show_file_dialog() -> Option<PathBuf>
{
  return FileDialog::new()
          .add_filter("image", &crate::image_coding::ImageFormat::available_formats())
          .set_directory("/")
          .save_file();
}
