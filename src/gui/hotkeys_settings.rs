use eframe::egui::{Ui, Event, ScrollArea};

use crate::itc::SettingsEvent;
use crate::hotkeys::{RegisteredHotkeys, HotkeyName, self};
use eframe::egui::KeyboardShortcut;
use std::sync::Arc;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, channel};

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
/// Si appoggia sul modulo RegisteredHotkeys: la struct sottostante realizza la parte di gui per settare le hotkeys,
/// registrando le modifiche all'interno di RegisteredHotkeys.
#[derive(Clone)]
pub struct HotkeysSettings
{
    state: HotkeySettingsState,
    registered_hotkeys: Arc<RegisteredHotkeys>,
    alert: Rc<RefCell<Option<String>>>,
    workers_channel: Rc<(Sender<Result<(), &'static str>>, Receiver<Result<(), &'static str>>)> 
}

impl HotkeysSettings
{
    /// Parametri:
    /// -   <b>alert</b>: smart pointer allo stato di errore globale di tutta l'applicazione;<br>
    /// -   <b>registered_hotkeys</b>: smart pointer contenente la struct RegisteredHotkeys che potrà
    ///     essere modificata da questo modulo. In particolare, il modulo richiederà degli updates
    ///     a RegisteredHotkeys, la quale li memorizzerà internamente, ma non li applicherà
    ///     fino a quando non viene chiamato il metodo <i>RegisteredHotkeys::update_changes(&self)</i>.
    ///     Il metodo viene infatti richiamato alla pressione del tasto "Save".<br>
    ///     Lo spazio per memorizzare le richieste di updates deve essere preparato opportunamente
    ///     richiamando il metodo <i>RegisteredHotkeys::prepare_for_updates(&self)</i> prima dell'esecuzione
    ///     di questo metodo.
    pub fn new(alert: Rc<RefCell<Option<String>>>, registered_hotkeys: Arc<RegisteredHotkeys>) -> Self
    {
        Self {state: HotkeySettingsState::Idle, registered_hotkeys, alert, workers_channel: Rc::new(channel())}
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
    pub fn update(&mut self, ui: &mut Ui)  -> SettingsEvent
    {
        let mut ret = SettingsEvent::Nil;

        ui.style_mut().spacing.button_padding = eframe::egui::vec2(12.0, 3.0);
        ui.separator();
        ui.label(eframe::egui::RichText::new("Hotkeys settings").heading());
        ui.separator();
        //controllo se c'è almeno un worker che ha ritornato errore
        while let Ok(r) = self.workers_channel.1.try_recv()
        {
            if let Err(e) = r {self.alert.borrow_mut().replace(e.to_string()); break;}
        }

        //controllo se è in corso la registrazione di una hotkey
        if let HotkeySettingsState::Registering(hn) = &mut self.state
        {
            let hn_clone = hn.clone(); //necessario clonare per poter distruggere il riferimento &mut creato sopra
            if let Some(new_hk) = self.registration_phase(ui)
            {
                let str_kh = new_hk.format(&eframe::egui::ModifierNames::NAMES, std::env::consts::OS == "macos" );
                self.state = HotkeySettingsState::Idle;
                self.registered_hotkeys.request_register(str_kh, hn_clone, self.workers_channel.0.clone());
            }
        }
        

        //gui
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
                                ui.style_mut().visuals.widgets.hovered.weak_bg_fill = eframe::egui::Color32::DARK_GREEN;
                                if ui.button("Save").clicked() {

                                    match self.registered_hotkeys.update_changes()
                                    {
                                        Ok(()) => { self.registered_hotkeys.serialize(); ret = SettingsEvent::Saved; },
                                        Err(e) => { self.alert.borrow_mut().replace(e); }
                                    };
                                    
                                }
                            });
                            ui.style_mut().visuals.widgets.hovered.weak_bg_fill = eframe::egui::Color32::RED;
                            if ui.button("Abort").clicked() {ret = SettingsEvent::Aborted;}
                            ui.add_space(10.0);
                            ui.heading("❓").on_hover_text("Hotkeys are combinations of keys pressed simultaneously with an associated action.\nThese combinations must be composed by at least one control button and only one key button.\nIf you press one of such hotkeys, the associated action is executed, even if this application is not in focus.\nRemember that these will eventually override other system's hotkeys (such as Ctrl+C) if you select the same combination of keys.")
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
                                ui.code("HELP: press at least one modifier and an alphabetic key.\nThe letter must be the last button to be pressed.\nWhen you press the letter, also the modifiers have to be pressed simultaneously.\nIf it doesn't work, make the pressure last longer.");
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
    /// - <b>hn<b>, identificativo della hotkey;
    /// - <b>value</b>, combinazione di tasti associata;
    /// Se è in corso la registrazione di un'altra hotkey, i bottoni di questa riga vengono disabilitati.
    fn row_gui(&mut self, ui: &mut Ui, hn: HotkeyName, value: String)
    {
        let mut label: String = hn.into();
        label.push_str(": ");
        ui.style_mut().visuals.widgets.hovered.weak_bg_fill = eframe::egui::Color32::from_rgb(0,140,250);
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
                            self.registered_hotkeys.request_unregister(hn)
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
            //la prima lettera premuta termina il processo di registrazione della hotkey
            if let Event::Key{key, pressed: _ , modifiers, repeat}  = event  
            {
                if modifiers.any() && !(*repeat)
                {
                    ret = Some(KeyboardShortcut::new(*modifiers, *key));
                }else {
                    self.alert.borrow_mut().replace("Invalid shortcut. Please follow the instructions.".to_string());
                    self.state = HotkeySettingsState::Idle;
                }
            }
        }
        ret
    }


}