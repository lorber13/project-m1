

use std::path::PathBuf;
use rfd::FileDialog;

struct FileD 
{

}
  
pub fn show_file_dialog() -> Option<PathBuf>
{
  let mut file = None;
  file = FileDialog::new()
          .add_filter("image", &crate::image_coding::ImageFormat::available_formats())
          .set_directory("/")
          .save_file();

  return file;
}
