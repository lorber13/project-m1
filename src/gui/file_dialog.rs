

use std::path::PathBuf;
use rfd::FileDialog;

use crate::image_coding::ImageFormat;

  
pub fn show_file_dialog(format: ImageFormat) -> Option<PathBuf>
{
  return FileDialog::new()
          .add_filter("image", &[format.into()])
          .set_directory("/")
          .save_file();
}
