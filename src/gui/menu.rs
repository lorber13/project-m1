use super::hotkeys_settings::HotkeysSettings;
use super::{capture_mode::CaptureMode, loading, save_settings::SaveSettings};
use crate::{
    hotkeys::RegisteredHotkeys,
    itc::{ScreenshotDim, SettingsEvent},
    screens_manager::ScreensManager,
};
use eframe::egui::{CentralPanel, Context, Ui};
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::{
    cell::RefCell,
    sync::{mpsc::TryRecvError, Arc},
};

pub enum MainMenuEvent {
    ScreenshotRequest(ScreenshotDim, f64),
    Nil,
}
/// Enum che descrive che cosa viene mostrato di fianco al bottone del menu'.
enum MainMenuState {
    CaptureMode(CaptureMode),
    SaveSettings(SaveSettings),
    LoadingHotkeysSettings(Receiver<()>),
    HotkeysSettings(HotkeysSettings),
}

/// Struct che descrive lo stato della porzione di gui che mostra il menu' di navigazione principale dell'applicazione, dal quale
/// si può accedere alle impostazioni, oppure avviare la cattura di uno screenshot.
/// Questa struct implementa una macchina a stati.
pub struct MainMenu {
    state: MainMenuState,
    alert: Rc<RefCell<Option<String>>>,
    screens_mgr: Arc<ScreensManager>,
    save_settings: Rc<RefCell<SaveSettings>>,
    registered_hotkeys: Arc<RegisteredHotkeys>,
}

impl MainMenu {
    /// Riceve come parametri gli smartpointer a parte dello stato globale dell'applicazione per poterlo modificare direttamente.
    pub fn new(
        alert: Rc<RefCell<Option<String>>>,
        screens_mgr: Arc<ScreensManager>,
        save_settings: Rc<RefCell<SaveSettings>>,
        registered_hotkeys: Arc<RegisteredHotkeys>,
    ) -> Self {
        Self {
            state: MainMenuState::CaptureMode(CaptureMode::new(screens_mgr.clone())),
            screens_mgr,
            alert,
            save_settings,
            registered_hotkeys,
        }
    }

    /// Mostra:
    /// - a sinsistra, un bottone "☰", che permette la visualizzazione del menu';
    /// - a destra, una schermata dipendente dalla voce del menu' selezionata.
    /// L'intero contenuto è disabilitato se il parametro enabled è settato a false.
    pub fn update(
        &mut self,
        enabled: bool,
        ctx: &Context,
        frame: &mut eframe::Frame,
    ) -> MainMenuEvent {
        let mut ret = MainMenuEvent::Nil;
        CentralPanel::default().show(ctx, |ui| {
            ui.add_enabled_ui(enabled, |ui| {
                ui.horizontal(|ui| {
                    ui.menu_button("☰", |ui| {
                        ui.vertical(|ui| {
                            if ui.button("Capture").clicked() {
                                ui.close_menu();
                                self.switch_to_main_window(frame);
                            }
                            ui.menu_button("Settings...", |ui| {
                                if ui.button("Save Settings").clicked() {
                                    ui.close_menu();
                                    self.switch_to_save_settings();
                                }

                                if ui.button("Hotkeys Settings").clicked() {
                                    ui.close_menu();
                                    self.switch_to_hotkeys_settings();
                                }
                            });
                        });
                    });

                    ui.vertical(|ui| {
                        ui.add_space(5.0);

                        match self.state {
                            MainMenuState::CaptureMode(_) => {
                                ret = self.show_main_window(ui, ctx, frame);
                            }
                            MainMenuState::SaveSettings(..) => {
                                self.show_save_settings(ui, frame);
                            }
                            MainMenuState::HotkeysSettings(..) => {
                                self.show_hotkeys_settings(ui, frame);
                            }
                            MainMenuState::LoadingHotkeysSettings(..) => {
                                self.load_hotkeys_settings(ctx);
                            }
                        }
                    });
                });
            });
        });

        ret
    }

    /*----------------MAIN WINDOW------------------------------------------ */

    /// Controlla qual'è l'attuale stato di main menu: se è già mostrata la schermata "capture mode", questo metodo non ha effetto.
    /// Altrimenti, modifica lo stato corrente.
    /// Nel nuovo stato viene memorizzata una nuova istanza di CaptureMode.
    fn switch_to_main_window(&mut self, _frame: &mut eframe::Frame) {
        match self.state {
            MainMenuState::CaptureMode(..) => (), //non c'è niente di nuovo da visualizzare
            _ => {
                self.state = MainMenuState::CaptureMode(CaptureMode::new(self.screens_mgr.clone()))
            }
        }
    }

    /// Chiama il metodo update() della struct CaptureMode memorizzata nello stato corrente.
    /// Gestisce i valori di ritorno di update(): se CaptureMode::update() ritorna i dettagli di una richiesta di
    /// screenshot, essi vengono incapsulati in MainMenuEvent::ScreenshotRequest.
    ///
    /// <h3>Panics:</h3>
    /// Se <i>self.state</i> è diverso da <i>MainMenuState::CaptureMode</i>.
    fn show_main_window(
        &mut self,
        ui: &mut Ui,
        ctx: &Context,
        frame: &mut eframe::Frame,
    ) -> MainMenuEvent {
        let mut ret = MainMenuEvent::Nil;
        if let MainMenuState::CaptureMode(ref mut cm) = self.state {
            //controllo l'utput della main window: se è diverso da None, significa che è stata creata una nuova richiesta di screenshot
            if let Some((area, delay)) = cm.update(ui, ctx, frame) {
                ret = MainMenuEvent::ScreenshotRequest(area, delay);
            }
        } else {
            unreachable!();
        }
        ret
    }

    //-----------------------------SAVE SETTINGS-------------------------------------------------------------------
    /// Se lo stato attuale della macchina a stati è già MainMenuState::SaveSettings, questo metodo non ha effetto.
    /// Altrimenti, modifica lo stato, memorizzando al suo interno una nuova istanza di SaveSettings, ottenuta
    /// clonando quella attuale dell'applicazione, così che il modulo save_settings modifichi soltanto una copia
    /// delle attuali impostazioni: non quelle originali.
    fn switch_to_save_settings(&mut self) {
        if crate::DEBUG {
            print!("DEBUG: switch to save settings");
        }
        match self.state {
            MainMenuState::SaveSettings(..) => (), //non c'è nulla di nuovo da visualizzare
            _ => {
                self.state = MainMenuState::SaveSettings(self.save_settings.borrow().clone());
            } //viene modificata una copia delle attuali impostazioni, per poter fare rollback in caso di annullamento
        }
    }

    /// Chiama il metodo update della struct SaveSettings memorizzata nello stato corrente.<br>
    /// Gestisce così il valore di ritorno di SaveSettings::update():
    /// - SettingsEvent::Saved, aggiorna lo stato globale dell'applicazione, sostituendolo con l'istanza di SaveSettings
    ///     memorizzata nello stato corrente, poi cambia lo stato di MainMenu in MainMenu::CaptureMode;
    /// - SettingsEvent::Aborted, e cambia lo stato di MainMenu in MainMenu::CaptureMode;
    /// - SettingsEvent::Nil, non fa nulla.
    ///
    /// <h3>Panics:</h3>
    /// Se <i>self.state</i> è diverso da <i>MainMenuState::SaveSettings</i>.
    fn show_save_settings(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        if let MainMenuState::SaveSettings(ss) = &mut self.state {
            match ss.update(ui) {
                SettingsEvent::Saved => {
                    self.save_settings.replace(ss.clone());
                    self.switch_to_main_window(frame);
                }
                SettingsEvent::Aborted => {
                    self.switch_to_main_window(frame);
                }
                SettingsEvent::Nil => (),
            }
        } else {
            unreachable!();
        }
    }

    //-----------------------------HOTKEYS SETTINGS-------------------------------------------------------------------
    /// Controlla qual'è l'attuale stato di main menu: se è già mostrata la schermata "hotkeys settings" oppure la schermata di loading (delle hotkey settings), questo metodo non ha effetto.<br>
    /// Altrimenti, richiama il metodo <i>HotkeySettings::prepare_for_updates()</i> e modifica lo stato corrente in LoadingHotkeySettings, memorizzando al suo interno
    /// il Receiver ritornato da <i>HotkeySettings::prepare_for_updates()</i>.
    fn switch_to_hotkeys_settings(&mut self) {
        if crate::DEBUG {
            print!("DEBUG: switch to hotkeys settings");
        }
        match self.state {
            MainMenuState::HotkeysSettings(..) | MainMenuState::LoadingHotkeysSettings(..) => (), //non c'è nulla di nuovo da visualizzare
            _ => {
                self.state = MainMenuState::LoadingHotkeysSettings(
                    self.registered_hotkeys.prepare_for_updates(),
                )
            } //viene modificata una copia delle attuali impostazioni, per poter fare rollback in caso di annullamento
        }
    }

    /// Gestisce la fase di caricamento di HotkeysSettings.<br>
    /// Esegue <i>try_recv()</i> sul receiver memorizzato nello stato corrente:
    /// - se non si sono verificati errori, cambia lo stato corrente in MainMenuState::HotkeysSettings, nel quale
    ///    memorizza una nuova istanza di HotkeysSettings;
    /// - se il canale associato al Receiver risulta chiuso, segnala errore attraverso lo stato di errore globale dell'applicazione;
    /// - se il canale è ancora vuoto, mostra uno spinner.<br>
    ///
    /// <h3>Panics:</h3>
    /// Se <i>self.state</i> è diverso da <i>MainMenuState::LoadingHotkeysSettings</i>.
    fn load_hotkeys_settings(&mut self, ctx: &Context) -> MainMenuEvent {
        let ret = MainMenuEvent::Nil;
        if let MainMenuState::LoadingHotkeysSettings(r) = &mut self.state {
            match r.try_recv() {
                Ok(()) => {
                    self.state = MainMenuState::HotkeysSettings(HotkeysSettings::new(
                        self.alert.clone(),
                        self.registered_hotkeys.clone(),
                    ))
                } //viene modificata una copia delle attuali impostazioni, per poter fare rollback in caso di annullamento
                Err(TryRecvError::Disconnected) => {
                    self.alert
                        .borrow_mut()
                        .replace("Loading failed".to_string());
                }
                Err(TryRecvError::Empty) => loading::show_loading(ctx),
            }
        } else {
            unreachable!();
        }
        ret
    }

    /// Siccome verrà visualizzata la schermata di impostazione delle hotkeys, disattiva temporaneamente l'ascolto delle hotkeys già registrate.
    /// Questo è necessario perchè, ad ogni refresh della gui, l'ascolto delle hotkeys è abilitato di default. Non si vuole tenerlo
    /// abilitato quando viene mostrata la schermata HotkeysSettings perchè potrebbe interferire con le operazioni di registrazione
    /// di nuove hotkeys. <br>
    /// Successivamente, esegue il metodo <i>HotkeysSettings::update()</i> e ne gestisce il valore di ritorno:
    /// - esce dalla schermata di impostazioni (cambiando lo stato in <i>MainMenuState::CaptureMode</i>) nel caso siano stati premuti i bottoni
    ///    "Save" o "Abort";
    /// - in caso di SettingsEvent::Nil, non compie nessuna operazione.
    ///
    /// <h3>Panics:</h3>
    /// Se <i>self.state</i> è diverso da <i>MainMenuState::LoadingHotkeysSettings</i>.
    fn show_hotkeys_settings(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        if let MainMenuState::HotkeysSettings(hs) = &mut self.state {
            self.registered_hotkeys.set_listen_enabled(false);
            match hs.update(ui) {
                SettingsEvent::Saved | SettingsEvent::Aborted => {
                    self.switch_to_main_window(frame);
                }
                SettingsEvent::Nil => (),
            }
        } else {
            unreachable!();
        }
    }
}
