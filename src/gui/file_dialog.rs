use rfd::FileDialog;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};

use crate::image_coding::ImageFormat;

/// Mostra, in una nuova finestra, un file dialog in modalità "Save".<br>
///
/// Parametri:
/// - <b>format</b>: l'estensione che verrà aggiunta al file;<br>
/// - <b>start_dir</b>: se specificata: directory inizialmente aperta, ma non vincolante sul path finale.<br>
///   Se non specificata, la directory inizialmente aperta sarà "/".<br>
///
/// Ritorna <b>Option</b>:
/// - None, se l'user ha annullato;<br>
/// - Some(PathBuf), se l'user ha dato un nome al file e premuto su "Save".<br>
///   Il path è assoluto e contiene il nome del file <b>senza estensione</b>.<br>
pub fn show_save_dialog(format: ImageFormat, start_dir: Option<String>) -> Option<PathBuf> {
    let dir = if let Some(s) = &start_dir {
        if !s.is_empty() && Path::new(&s).exists() {
            s
        } else {
            "/"
        }
    } else {
        "/"
    };

    FileDialog::new()
        .add_filter("image", &[format.into()])
        .set_directory(dir)
        .save_file()
}

/// Mostra un dialog con la possibilità di selezionare una cartella.<br>
///
/// Parametri:
/// - <b>start_dir</b>: se specificata: directory inizialmente aperta, ma non vincolante sul path finale.<br>
///   Se non specificata, la directory inizialmente aperta sarà "/".<br>
///
/// Ritorna <b>Option</b>:
/// - None, se l'user ha annullato;<br>
/// - Some(PathBuf), se l'user ha selezionato la cartella e premuto su "Save".<br>
///   Il path è assoluto.
pub fn show_directory_dialog(start_dir: Option<String>) -> Option<PathBuf> {
    let dir = if let Some(s) = start_dir {
        if !s.is_empty() && Path::new(&s).exists() {
            s
        } else {
            "/".to_string()
        }
    } else {
        "/".to_string()
    };

    FileDialog::new().set_directory(dir).pick_folder()
}

pub fn start_thread_directory_dialog(start_dir: Option<String>) -> Receiver<Option<PathBuf>> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let _ = tx.send(show_directory_dialog(start_dir));
    });
    rx
}
