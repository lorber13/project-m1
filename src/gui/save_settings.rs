

use eframe::egui::{self, ScrollArea};
use crate::itc::SettingsEvent;
use chrono::Local;
use std::cell::RefCell;
use super::file_dialog;
use std::rc::Rc;
use std::cell::Cell;
use crate::image_coding::ImageFormat;
use std::path::PathBuf;
use crate::DEBUG;
use std::io::Write;
use std::sync::mpsc::{channel, Receiver};


#[derive(Clone)]
struct DefaultDir
{
    enabled: bool,
    path: String
}

#[derive(Clone, Copy)]
enum DefaultNameMode
{
    OnlyName,
    Counter(u64),
    Timestamp
}

impl Into<&'static str> for DefaultNameMode
{
    fn into(self) -> &'static str
    {
        match self
        {
            Self::OnlyName => "Default name",
            Self::Counter(..) => "Default name + incremental number",
            Self::Timestamp => "Default name + timestamp"
        }
    }
}

impl PartialEq for DefaultNameMode
{
    fn eq(&self, other: &Self) -> bool {

        matches!((self, other), (DefaultNameMode::Counter(..), DefaultNameMode::Counter(..)) | (DefaultNameMode::Timestamp, DefaultNameMode::Timestamp))
    }
}


#[derive(Clone)]
struct DefaultName
{
    enabled: bool,
    mode: Cell<DefaultNameMode>,
    name: String
}

/// Memorizza lo stato del componente della gui dedicato alle impostazioni di salvataggio delle immagini, 
/// costituendo contemporaneamente una struttura dati per memorizzare queste informazioni anche quando
/// il componente gui non è mostrato.
#[derive(Clone)]
pub struct SaveSettings
{
    default_dir: DefaultDir,
    default_name: DefaultName,
    pub copy_on_clipboard: bool, 
    /// Riferimento allo stato di errore globale dell'applicazione.
    alert: Rc<RefCell<Option<String>>>
}

impl SaveSettings
{
    pub fn new(alert: Rc<RefCell<Option<String>>>) -> Self
    {
        Self {default_dir: DefaultDir { enabled: false, path: "".to_string() }, 
                default_name: DefaultName { enabled: false, name: "".to_string(), mode: Cell::new(DefaultNameMode::Timestamp),},
                copy_on_clipboard: true,
                alert
            }
    }

    /// Controlla quali sono le impostazioni di salvataggio di default attualmente in uso. Sia la directory di default che il
    /// nome di default possono essere abilitati o disabilitati, quindi esistono quattro possibili casistiche:<br>
    ///
    /// (default_dir, default_name) =
    /// - <i>(Some(..), Some(..))</i>: non è necessario mostrare all'user nessun file dialog perchè il path di salvataggio è
    ///     già conosciuto;
    /// - <i>(None, Some(..))</i>: viene mostrato un directory dialog;
    /// - <i>(Some(..), None)</i>: viene mostrato un file dialog che di default apre la default_dir, ma potenzialmente l'user
    ///     potrebbe modificare a piacere la cartella di salvataggio in questa fase;
    /// - <i>(None, None)</i>: viene mostrato un file dialog che di default apre la cartella root.
    ///
    /// Se l'user conferma l'operazione di selezione tramite file dialog, oppure se si è presentata la prima situazione sopra
    /// descritta, il metodo avvia il thread che realizza il salvataggio al path ottenuto. Poi, memorizza nello stato corrente
    /// l'estremità <i>Receiver</i> del canale di comunicazione con tale thread. Lo stato cambia quindi in <i>EnumGuiState::Saving</i>.<br>
    ///
    /// Se l'user annulla l'operazione di selezione tramite file dialog, la gui continua a mostrare la schermata EditImage.
    pub fn compose_output_file_path(&self, format: ImageFormat) -> Receiver<Option<PathBuf>> {

        let dd_opt = self.get_default_dir();
        let dn_opt = self.get_default_name();
        let (tx, rx) = channel();
        std::thread::spawn(move||{
            match (dd_opt, dn_opt) {
                (Some(dp), Some(dn)) => {
                    let mut pb = PathBuf::from(dp);
                    let ext: &str = format.into();
                    pb.push("temp");
                    pb.set_file_name(dn);
                    pb.set_extension(ext);
                    let _ = tx.send(Some(pb));
                    return;
                }
    
                (None, Some(dn)) => {
                    let dir_opt = file_dialog::show_directory_dialog(None);
                    if let Some(mut pb) = dir_opt {
                        if DEBUG {
                            let _ = writeln!(
                                std::io::stdout(),
                                "DEBUG: dir picker return = {}",
                                pb.display()
                            );
                        }
    
                        let ext: &str = format.into();
                        pb.set_file_name(dn);
                        pb.set_extension(ext);
                        let _ = tx.send(Some(pb));
                        return;
                    }
                },
    
                (Some(dp), None) => {
                    let dir_opt = file_dialog::show_save_dialog(format, Some(dp));
                    if let Some(mut pb) = dir_opt {
                        let ext: &str = format.into();
                        pb.set_extension(ext);
                        let _ = tx.send(Some(pb));
                        return;
                    }
                },
    
                (None, None) => {
                    let dir_opt = file_dialog::show_save_dialog(format, None);
                    if let Some(mut pb) = dir_opt {
                        let ext: &str = format.into();
                        pb.set_extension(ext);
                        let _ = tx.send(Some(pb));
                        return;
                    }
                }
            }
            let _ = tx.send(None);
        });

        rx
    }

    /// Mostra, all'interno di una ScrollArea orizzontale, una schermata divisa in quattro sezioni:
    /// 1. form relativo alla directory di default;
    /// 2. form relativo al nome di default;
    /// 3. form relativo alla copia negli appunti;
    /// 4. bottoni per salvataggio o annullamento.
    /// 
    /// <b>Sezione 1:</b> contiene un input text per specificare il path a mano e un bottone per aprire un directory dialog.<br>
    /// 
    /// <b>Sezione 2:</b> 
    /// - input text per specificare il nome che di default è assegnato ad ogni immagine salvata;
    /// - combobox per aggiungere opzionalmente un numero incrementale o il timestamp.
    /// 
    /// <b>Sezione 3:</b> checkbox per attivare/disattivare la copia automatica dell'immagine negli appunti.<br>
    /// <i>NOTA: la copia viene fatta prima della modifica dell'immagine, negli appunti ci sarà solo l'immagine non modificata</i>
    /// 
    /// <b>Sezione 4:</b> 
    /// - bottone "Save": se premuto, il metodo ritorna <i>SettingsEvent::Saved</i>;
    /// - bottone "Abort": se premuto, il metodo ritorna <i>SettingsEvent::Aborted</i>;
    /// - etichetta che mostra un punto interrogativo: mostra un tooltip con istruzioni utili per questa schermata.
    pub fn update(&mut self, ui: &mut egui::Ui) -> SettingsEvent
    {
        let mut ret = SettingsEvent::Nil;

        ui.set_height(ui.available_height());
        ScrollArea::new([true, false]).show(ui, |ui|
        {
            ui.separator();
            ui.label(eframe::egui::RichText::new("Save settings").heading());
            ui.separator();
            ui.add(egui::Checkbox::new(&mut self.default_dir.enabled, "Save all screenshot in a default directory"));
            ui.style_mut().spacing.button_padding = eframe::egui::vec2(12.0, 3.0);
            ui.add_enabled_ui(self.default_dir.enabled, |ui|
            {
                ui.horizontal(|ui|
                        {
                            ui.add(egui::TextEdit::singleline(&mut self.default_dir.path));
                            if ui.button("📁").clicked()
                            {
                                ret = SettingsEvent::OpenDirectoryDialog;
                            }
                        });
            });
            ui.separator();

            ui.add(egui::Checkbox::new(&mut self.default_name.enabled, "Default file name"));
            ui.add_enabled_ui(self.default_name.enabled, |ui|
            {
                ui.horizontal(|ui| {
                    let former = self.default_name.name.clone();
                    let res1 = ui.add(egui::TextEdit::singleline(&mut self.default_name.name));
                    if res1.lost_focus() && self.default_name.name != former 
                    {
                        if let DefaultNameMode::Counter(_) = self.default_name.mode.get()
                        {
                            self.default_name.mode.replace(DefaultNameMode::Counter(0));
                        }
                    }

                    egui::ComboBox::from_label("Naming Mode") //prova di menù a tendina per scegliere se fare uno screen di tutto, oppure per selezionare un rettangolo
                    .selected_text(<DefaultNameMode as Into<&'static str>>::into(self.default_name.mode.get()))
                    .show_ui(ui, |ui|{
                        ui.style_mut().wrap = Some(false);
                        ui.set_min_width(60.0);
                        ui.selectable_value(&mut self.default_name.mode.get(), DefaultNameMode::OnlyName, <DefaultNameMode as Into<&'static str>>::into(DefaultNameMode::OnlyName))
                        .on_hover_text("If exists another file with the same name in the dir, it will be overwritten.");
                        ui.selectable_value(&mut self.default_name.mode.get(), DefaultNameMode::Counter(0), <DefaultNameMode as Into<&'static str>>::into(DefaultNameMode::Counter(0)))
                        .on_hover_text("If exists another file with the same name in the dir, it will be overwritten.");
                        ui.selectable_value(&mut self.default_name.mode.get(), DefaultNameMode::Timestamp, <DefaultNameMode as Into<&'static str>>::into(DefaultNameMode::Timestamp))
                        .on_hover_text("timestamp format: YYYY-MM-DD_HH-MM-SS");
                    });
                    
                });


            });
            ui.separator();

            ui.add_space(10.0);
            ui.checkbox(&mut self.copy_on_clipboard, "Copy on clipboard")
            .on_hover_text("When you acquire a screenshot, the acquired image is automatically copied in you clipboard.\nNote that modifications to the image performed after the acquire phase are not included.");
            ui.separator();


            ui.add_space(20.0);
            ui.horizontal(|ui|
                {
                    ui.style_mut().visuals.widgets.hovered.weak_bg_fill = egui::Color32::DARK_GREEN;
                    if ui.button("Save").clicked() {
                        if self.default_dir.enabled && ( self.default_dir.path.is_empty() || !std::path::Path::new(&self.default_dir.path).exists())
                        {
                            if crate::DEBUG {println!("Found an invalid dir path");}
                            self.alert.borrow_mut().replace("Invalid default directory path.".to_string());
                        }else if self.default_name.enabled && self.default_name.mode.get() == DefaultNameMode::OnlyName && self.default_name.name.is_empty()
                        {
                            self.alert.borrow_mut().replace("Default name cannot be empty.".to_string());
                        }else {
                            ret = SettingsEvent::Saved;
                        }
                    }
                    ui.style_mut().visuals.widgets.hovered.weak_bg_fill = egui::Color32::RED;
                    if ui.button("Abort").clicked() {ret = SettingsEvent::Aborted;}
                })
        
        });


        ret

    }


    /// Ritorna:
    /// - <i>None</i>, se il salvataggio in una cartella di default è disabilitato (<i>self.default_dir.enabled == false</i>), 
    ///     oppure nullo o invalido (<i>NOTA: un path potrebbe essere diventato invalido se l'albero del file system è cambiato
    ///     dopo che le SaveSettings sono state salvate correttamente</i>);
    /// - <i>Some()<i>, contenente il path scritto sotto forma di stringa altrimenti.
    pub fn get_default_dir(&self) -> Option<String>
    {
        if !self.default_dir.enabled || self.default_dir.path.len() == 0 
            || !std::path::Path::new(&self.default_dir.path).exists() {return None;}

        Some(self.default_dir.path.clone())
    }

    /// Ritorna:
    /// - <i>None<i>, se il salvataggio con un nome di default è disabilitato (<i>self.default_name.enabled == false</i>);
    /// - <i>Some()<i>, contenente il nome per il prossimo file da salvare altrimenti.<br>
    ///     - Nel caso <i>self.default_name.mode</i> sia <i>DefaultNameMode::Timestamp</i>, calcola e formatta il timestamp
    ///         corrente e lo concatena alla stringa del default name.<br>
    ///     - Nel caso <i>self.default_name.mode</i> sia <i>DefaultNameMode::Timestamp</i>, concatena alla stringa del default name
    ///         il valore del contantore, poi lo incrementa.<br> 
    ///         (<i>NOTA: il campo self.default_name.mode è stato inserito in 
    ///         una Cell per permettere a questo metodo di ricevere come parametro self com riferimento non mutabile, nascondendo
    ///         così il meccanismo di incremento interno del contatore</i>)
    pub fn get_default_name(&self) -> Option<String>
    {
        if !self.default_name.enabled {return None;}
    
        match self.default_name.mode.get()
        {
            DefaultNameMode::OnlyName =>
            {
                Some(self.default_name.name.clone())
            },

            DefaultNameMode::Counter(c) => 
            {
                let str = format!("{}{}", self.default_name.name, c);
                self.default_name.mode.replace(DefaultNameMode::Counter(c+1));
                Some(str)
            },

            DefaultNameMode::Timestamp =>
            {
                const TIMESTAMP_FMT: &str = "%Y-%m-%d_%H%M%S";
                let str = format!("{}{}", self.default_name.name, Local::now().format(TIMESTAMP_FMT));
                Some(str)
            }
        }

        
    }

    pub fn set_default_directory(&mut self, dir: String)
    {
        if self.default_dir.enabled {
            if crate::DEBUG { println!("\nsetting default dir: {}", dir); }       
            self.default_dir.path = dir;
        }   
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn create_ss() -> SaveSettings
    {
        let _ = std::fs::remove_dir("./dd");
        std::fs::create_dir("./dd").unwrap();
        let mut ss = SaveSettings::new(Rc::new(RefCell::new(None)));
        ss.default_dir = DefaultDir{path: PathBuf::from("dd").to_str().unwrap().to_string(), enabled: true};
        ss.default_name = DefaultName{enabled: true, name: "dn".to_string(), mode: Cell::new(DefaultNameMode::OnlyName)};
        ss
    }

    #[test]
    fn compose_output_file_path_test()
    {
        let ss = create_ss();

        let res = ss.compose_output_file_path(ImageFormat::GIF).recv().unwrap();
        if let Some(path) = res
        {
            assert!(path.extension().unwrap().to_str().unwrap() == "Gif");
            assert!(path.file_name().unwrap().to_str().unwrap() == "dn.Gif");
            assert!(path.starts_with("dd"));
        }else {
            assert!(res.is_some());
        }

        std::fs::remove_dir("./dd").unwrap();
    }

    #[test]
    fn get_default_name_test()
    {
        let mut ss = create_ss();
        ss.default_name.mode = Cell::new(DefaultNameMode::Counter(0));
        assert!(ss.get_default_name().unwrap() == "dn0");
        assert!(ss.get_default_name().unwrap() == "dn1");
        assert!(ss.get_default_name().unwrap() == "dn2");
    }
}
