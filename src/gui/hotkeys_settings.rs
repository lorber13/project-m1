use eframe::egui::{Ui, Event, Context, ScrollArea};

use crate::itc::SettingsEvent;
use crate::hotkeys::{RegisteredHotkeys, HotkeyName, self};
use eframe::egui::KeyboardShortcut;
use std::io::stderr;
use std::io::Write;
use std::sync::Arc;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
enum HotkeySettingsState
{
    Idle,
    Registering(HotkeyName)
}

impl PartialEq for HotkeySettingsState
{
    fn eq(&self, rhs: &Self) -> bool
    {
        match (self, rhs)
        {
            (HotkeySettingsState::Idle, HotkeySettingsState::Idle) => true,
            (HotkeySettingsState::Registering(hn1), HotkeySettingsState::Registering(hn2)) => hn1 == hn2,
            _ => false
        }
    }
}

/// Stato della parte della gui che visualizza la schermata di impostazione delle Hotkeys.<br>
#[derive(Clone)]
pub struct HotkeysSettings
{
    state: HotkeySettingsState,
    registered_hotkeys: Arc<RegisteredHotkeys>,
    alert: Rc<RefCell<Option<&'static str>>>, 
}

impl HotkeysSettings
{
    /// Parametri:
    /// -   <b>alert</b>: smartpointer allo stato di errore globale di tutta l'applicazione;<br>
    /// -   <b>registered_hotkeys</b>: smartpointer contenente la struct RegisteredHotkeys che potrà
    ///     essere modificata da questo modulo (conviene passare a questo modulo una copia delle RegisteredHotkeys
    ///     memorizzate a livello globale, così che eventuali operazioni di rollback o di gestione errori
    ///     non lascino le RegisteredHotkeys originali in uno stato incoerente).
    pub fn new(alert: Rc<RefCell<Option<&'static str>>>, registered_hotkeys: Arc<RegisteredHotkeys>) -> Self
    {
        Self {state: HotkeySettingsState::Idle, registered_hotkeys, alert}
    }

    /// Mostra, per ogni possibile hotkey in RegisteredHotkeys, un form per la sua configurazione.<br>
    /// Ogni form è disabilitato se è in corso la registrazione di un'altra hotkey.<br>
    /// Durante la registrazione di una hotkey, viene visualizzato un messaggio con le istruzioni per l'user.
    /// Nel caso venga ricevuta in input una hotkey valida, questo metodo provvede a richiederne la registrazione
    /// in RegisteredHotkeys.<br>
    /// 
    /// Ritorna <b>SettingsEvent</b>:
    /// -   SettingsEvent::Save, se è stato premuto il bottone "Save";
    /// -   SettingsEvent::Abort, se è stato premuto il bottone "Abort";
    /// -   SettingsEvent::Nil altrimenti.
    pub fn update(&mut self, ui: &mut Ui, ctx: &Context) -> SettingsEvent
    {
        let mut ret = SettingsEvent::Nil;

        //controllo se è in corso la registrazione di una hotkey
        if let HotkeySettingsState::Registering(hn) = &mut self.state
        {
            let hn_clone = hn.clone(); //necessario clonare per poter distruggere il riferimento &mut creato sopra
            if let Some(new_hk) = self.registration_phase(ui)
            {
                let str_kh = new_hk.format(&eframe::egui::ModifierNames::NAMES, std::env::consts::OS == "macos" );
                self.state = HotkeySettingsState::Idle;
                if let Err(e) = self.registered_hotkeys.register(str_kh, hn_clone)
                {
                    self.alert.borrow_mut().replace(e);
                }
            }
        }
        
        ScrollArea::new([true, false]).show(ui, |ui|
        {
            ui.vertical(|ui|
                {
    
                    for i in 0..hotkeys::N_HOTK
                    {
                        let value = match self.registered_hotkeys.get_hotkey_string(HotkeyName::from(i)) {Some(str) => str.clone(), None => String::from("")};
    
                        self.row_gui(ui, HotkeyName::from(i), value);
                    }
    
                    ui.separator();
                    ui.add_space(30.0);
                    ui.horizontal(|ui|
                        {
                            //non si può salvare se è in corso la registrazione di una hotkey
                            ui.add_enabled_ui(self.state == HotkeySettingsState::Idle, |ui|
                            {
                                if ui.button("Save").clicked() {
                                    ret = SettingsEvent::Saved;
                                }
                            });
                            if ui.button("Abort").clicked() {ret = SettingsEvent::Aborted;}
                        });
    
                    //messaggio di help che viene visualizzato mentre si sta registrando una hotkey
                    if self.state != HotkeySettingsState::Idle
                    {
                        ui.vertical(|ui|
                        {
                            ui.add_space(50.0);
                            ui.horizontal(|ui|
                            {
                                ui.heading("?");
                                ui.code("HELP: press at least one modifier and an alphabetic key.\nThe letter must be the last button to be pressed.\nWhen you press the letter, also the modifiers have to be pressed simoultaneously.\nIf it doesn't work, make the pressure last longer.");
                            })
                        });
                    }
                });
        });

        ret
    }

    /// Mostra una riga con etichetta (della hotkey), stringa che rappresenta la combinazione di tasti, bottoni per 
    /// avviare la registrazione o per eliminare la hotkey.<br>
    /// Parametri:
    /// - <b>hn<b>, identificaivo della hotkey;
    /// - <b>value</b>, combinazione di tasti associata;
    /// Se è in corso la registrazione di un'altra hotkey, i bottoni di questa riga vengono disabilitati.
    fn row_gui(&mut self, ui: &mut Ui, hn: HotkeyName, value: String)
    {
        let mut label: String = hn.into();
        label.push_str(": ");

        ui.add_enabled_ui(self.state == HotkeySettingsState::Idle || self.state == HotkeySettingsState::Registering(hn), |ui|
        {
            ui.horizontal(|ui|
                {
                    ui.label(label);
                    ui.label(value);
                    
                    ui.with_layout(eframe::egui::Layout::right_to_left(eframe::egui::Align::TOP), |ui|
                    {   

                        if ui.button("Delete hotkey").clicked()
                        {
                            if let Err(e) = self.registered_hotkeys.unregister(hn)
                            {
                                self.alert.borrow_mut().replace("Error: unable to complete the operation");
                                let _= write!(stderr(), "Err = {}", e);
                            }
                        } 

                        
                        if ui.button("Set hotkey").clicked()
                        {
                            //avvia la registrazione della hotkey
                            self.state = HotkeySettingsState::Registering(hn);
                        }
 
                    });
                    
                });
        });
        
    }

    /// Controlla tutti gli input events. Se tra questi c'è la pressione di un tasto lettera, controlla se contemporaneamente
    /// sono premuti altri tasti di controllo:
    /// - in caso affermativo, costruisce un oggetto <b>KeyboardShortcut</b> con i tasti premuti e lo ritorna;
    /// - in caso negativo, segnala, inserendo una stringa nell'alert globale, che la combinazione non è valida 
    /// (<i>una hotkey valida è composta da almeno un tasto di controllo e un'unica lettera</i>)
    fn registration_phase(&mut self, ui: &mut Ui) -> Option<KeyboardShortcut>
    {
        let mut ret = None;
        let events = ui.input(|i| {i.events.clone()});
        for event in &events
        {
            match event
            {
                //la prima lettera premuta termina il processo di registrazione della hotkey
                Event::Key{key, pressed: _ , modifiers, repeat}  =>  
                {
                      if modifiers.any() && *repeat == false
                      {
                        ret = Some(KeyboardShortcut::new(modifiers.clone(), key.clone()));
                      }else {
                          self.alert.borrow_mut().replace("Invalid shortcut. Please follow the instructions.");
                          self.state = HotkeySettingsState::Idle;
                      }
                }
                _ => ()
            }
        }
        ret
    }


}