

use eframe::egui::{self, ScrollArea};
use crate::itc::SettingsEvent;
use chrono::Local;
use std::cell::RefCell;
use super::file_dialog;
use std::rc::Rc;
use std::cell::Cell;


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
        match (self, other)
        {
            (DefaultNameMode::Counter(..), DefaultNameMode::Counter(..)) | (DefaultNameMode::Timestamp, DefaultNameMode::Timestamp) => true,
            _ => false
        }
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
            ui.add(egui::Checkbox::new(&mut self.default_dir.enabled, "Save all screenshot in a default directory"));
            ui.add_enabled_ui(self.default_dir.enabled, |ui|
            {
                ui.horizontal(|ui|
                        {
                            ui.add(egui::TextEdit::singleline(&mut self.default_dir.path));
                            if ui.button("📁").clicked()
                            {
                                match file_dialog::show_directory_dialog(Some(&self.default_dir.path))
                                {
                                    None => (),
                                    Some(pb) => self.default_dir.path = String::from(pb.to_str().unwrap())
                                }
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
                    if ui.button("Save").clicked() {
                        if self.default_dir.enabled && ( self.default_dir.path.len() == 0 || !std::path::Path::new(&self.default_dir.path).exists())
                        {
                            if crate::DEBUG {println!("Found an invalid dir path");}
                            self.alert.borrow_mut().replace("Invalid default directory path.".to_string());
                        }else if self.default_name.enabled && self.default_name.mode.get() == DefaultNameMode::OnlyName && self.default_name.name.len() == 0
                        {
                            self.alert.borrow_mut().replace("Default name cannot be empty.".to_string());
                        }else {
                            ret = SettingsEvent::Saved;
                        }
                    }
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
                const TIMESTAMP_FMT: &'static str = "%Y-%m-%d_%H%M%S";
                let str = format!("{}{}", self.default_name.name, Local::now().format(TIMESTAMP_FMT));
                Some(str)
            }
        }

        
    }
}
