

use std::path::PathBuf;
use rfd::FileDialog;
use std::path::Path;

use crate::image_coding::ImageFormat;

  
pub fn show_save_dialog(format: &ImageFormat) -> Option<PathBuf>
{
  return FileDialog::new()
          .add_filter("image", &[(*format).into()])
          .set_directory("/")
          .save_file();
}

pub fn show_directory_dialog(start_dir: &str) -> Option<PathBuf>
{
  let mut fd = FileDialog::new();
  if start_dir.len() != 0 && Path::new(&start_dir).exists()
  {
    fd = fd.set_directory(start_dir);
  }
  return  fd.pick_folder();
}