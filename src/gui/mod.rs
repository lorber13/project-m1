/*
La gui, a causa delle limitazioni imposte da eframe, deve essere eseguita solo nel thread principale.
Questo modulo è disegnato per permettere al thread che esegue la gui di non iniziare mai attese bloccanti.

La gui è quindi basata su una macchina a stati e le varianti della EnumGuiState incapsulano le variabili
con i dettagli di ciascuno stato.
In particolare, se una variante incapsula un Receiver, allora essa rappresenta uno stato di attesa
della gui, che fa busy waiting con try_recv(). Si noti che il design della sincronizzazione con altri
thread, appena descritto, non aggiunge overhead perché asseconda il funzionamento dell'event loop della gui, che continua a ridisegnarsi.

Lo stato della gui è memorizzato dentro la struct GlobalGuiState assieme ad altre informazioni globali.
 */

mod capture_mode;
mod edit_image;
mod error_alert;
pub mod file_dialog;
mod hotkeys_settings;
mod loading;
mod menu;
mod rect_selection;
mod save_settings;

use self::edit_image::FrameEvent;
use self::menu::MainMenuEvent;
use crate::gui::loading::show_loading;
use crate::hotkeys::{self, HotkeyName, RegisteredHotkeys};
use crate::image_coding::{start_thread_copy_to_clipboard, ImageFormat};
use crate::itc::ScreenshotDim;
use crate::{image_coding, screens_manager, DEBUG};
use edit_image::EditImage;
use eframe::egui::Rect;
use image::{ImageError, RgbaImage};
use menu::MainMenu;
use rect_selection::RectSelection;
use save_settings::SaveSettings;
use std::cell::RefCell;
use std::fmt::Formatter;
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

/// Possibili valori dello stato interno della macchina a stati realizzata dalla struct <i>GlobalGuiState</i>.
pub enum EnumGuiState {
    MainMenu(MainMenu),
    WaitingForDelay(Option<JoinHandle<()>>, ScreenshotDim),
    LoadingRectSelection(Receiver<Result<RgbaImage, &'static str>>),
    RectSelection(RectSelection),
    LoadingEditImage(Receiver<Result<RgbaImage, &'static str>>),
    EditImage(EditImage),
    Saving(Receiver<Result<String, ImageError>>),
}

impl std::fmt::Debug for EnumGuiState {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            EnumGuiState::MainMenu(_) => write!(f, "EnumGuiState::MainMenu"),
            EnumGuiState::WaitingForDelay(..) => write!(f, "EnumGuiState::WaitingForDelay"),
            EnumGuiState::LoadingRectSelection(..) => {
                write!(f, "EnumGuiState::LoadingRectSelection")
            }
            EnumGuiState::RectSelection(..) => write!(f, "EnumGuiState::RectSelection"),
            EnumGuiState::EditImage(..) => write!(f, "EnumGuiState::EditImage"),
            EnumGuiState::LoadingEditImage(_) => write!(f, "EnumGuiState::LoadingEdiImage"),
            EnumGuiState::Saving(_) => write!(f, "EnumGuiState::Saving"),
        }
    }
}

/// Memorizza lo stato globale della dell'applicazione.
pub struct GlobalGuiState {
    /// Stato corrente della macchina a stati (quindi, dell'intera applicazione).
    state: EnumGuiState,
    /// Stato di errore globale dell'applicazione.
    alert: Rc<RefCell<Option<String>>>,
    /// Gestore degli schermi rilevati dal sistema.
    screens_manager: Arc<screens_manager::ScreensManager>,
    /// Impostazioni di salvataggio automatico delle immagini.
    save_settings: Rc<RefCell<SaveSettings>>,
    /// Gestore delle hotkeys registrate.
    registered_hotkeys: Arc<RegisteredHotkeys>,
    /// Contiene Some() se è stato lanciato un worker per copiare dati sulla clipboard.
    clipboard: Option<Receiver<Result<(), arboard::Error>>>,
    /// Receiver del canale di comunicazione con il thread dedicato all'ascolto delle hotkeys
    hotkey_receiver: Option<Receiver<HotkeyName>>,
    ///Se != None, allora l'applicazione ha avviato un thread worker per costruire il path di destinazione
    /// prima del salvataggio dell'immagine: la finestra principale deve essere mostrata ma disabilitata
    pending_save_request: Option<(Receiver<Option<PathBuf>>, RgbaImage)>,
    ///Se != None, allora l'applicazione è in attesa che l'utente chiuda il file dialog
    directory_dialog_receiver: Option<Receiver<Option<PathBuf>>>,
}

impl GlobalGuiState {
    /// Crea una nuova istanza della macchina a stati, del gestore delle hotkeys e degli schermi.
    /// Lo stato iniziale è <i>EnumGuiState::MainMenu</i>.
    fn new() -> Self {
        let alert: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let registered_hotkeys = RegisteredHotkeys::new();
        let save_settings = Rc::new(RefCell::new(SaveSettings::new(alert.clone())));
        let screens_manager = screens_manager::ScreensManager::new(150);
        GlobalGuiState {
            state: EnumGuiState::MainMenu(MainMenu::new(
                alert.clone(),
                screens_manager.clone(),
                save_settings.clone(),
                registered_hotkeys.clone(),
            )),
            alert,
            screens_manager,
            save_settings,
            registered_hotkeys,
            clipboard: None,
            hotkey_receiver: None,
            pending_save_request: None,
            directory_dialog_receiver: None,
        }
    }

    /// Modifica lo stato della macchina a stati in <i>EnumGuiState::MainMenu</i>, in cui memorizza una nuova istanza di MainMenu.
    fn switch_to_main_menu(&mut self, frame: &mut eframe::Frame) {
        frame.set_decorations(true);
        frame.set_fullscreen(false);
        frame.set_maximized(false);
        frame.set_window_size(eframe::egui::Vec2::new(500.0, 300.0));
        frame.set_visible(true);
        self.state = EnumGuiState::MainMenu(MainMenu::new(
            self.alert.clone(),
            self.screens_manager.clone(),
            self.save_settings.clone(),
            self.registered_hotkeys.clone(),
        ));
    }

    /// Esegue il metodo <i>MainMenu::update()</i>, a cui passa il parametro enabled.
    /// Gestisce il caso in cui <i>MainMenu::update()</i> restituisca <i>MainMenuEvent::ScreenshotRequest</i>, richiamando
    /// <i>Self::start_wait_delay()</i> per soddisfare la richiesta dopo il delay impostato.
    /// Oppure <i>MainMenuEvent:: OpenDirectoryDialog</i>, richiamando il metodo per rendere disabilitata la finestra
    ///corrente e aprire il file dialog.
    ///  
    /// <h3>Panics:</h3>
    /// Nel caso <i>self.state</i> sia diverso da <i>EnumGuiState::MainMenu</i>.
    fn show_main_menu(
        &mut self,
        ctx: &eframe::egui::Context,
        frame: &mut eframe::Frame,
        enabled: bool,
    ) {
        if let EnumGuiState::MainMenu(m) = &mut self.state {
            match m.update(enabled, ctx) {
                MainMenuEvent::ScreenshotRequest(sd, d) => self.start_wait_delay(d, sd, frame, ctx),
                MainMenuEvent::OpenDirectoryDialog => self.open_directory_dialog(),
                MainMenuEvent::Nil => (),
            }
        } else {
            unreachable!();
        }
    }

    ///Lancia il thread che gestisce il file dialog per la scelta di una cartella.
    /// Il file dialog mostratò sarà inizialmente aperto nel path indicato come default directory nelle save settings.
    /// Salva il <i>Receiver</i> del canale con tale thread nello stato globale.
    fn open_directory_dialog(&mut self) {
        let rx = file_dialog::start_thread_directory_dialog(
            self.save_settings.borrow().get_default_dir(),
        );
        self.directory_dialog_receiver = Some(rx);
    }

    ///A seconda dello stato attuale della gui, gestisce diversamente l'attesa che il thread
    /// che gestisce il directory dialog invii un risultato sul canale.
    ///
    ///<h3>Panics:</h3>
    /// se lo stato attuale della gui non è tra quelli per i quali è prevista la gestione del directory dialog.
    fn wait_directory_dialog(&mut self) {
        if let EnumGuiState::MainMenu(m) = &mut self.state {
            m.wait_directory_dialog(&mut self.directory_dialog_receiver);
        } else {
            unreachable!();
        }
    }

    /// Data una richiesta di screenshot, rende invisibile l'applicazione e
    /// lancia il thread che esegue una sleep.<br/>
    /// La durata della sleep corrisponde a:
    /// - il parametro <i>d</i>, che corrisponde al delay impostato dall'utente, se <i>d>0</i>;
    /// - il tempo impiegato alle animazioni del sistema operativo per rendere invisibile
    /// la finestra, se <i>d==0</i>.
    ///
    /// <i>NOTA: l'applicazione rimane nello stato WaitingForDelay per un frame soltanto,
    /// ma il passaggio è comunque necessario per eseguire il repaint e quindi
    /// fare diventare l'applicazione invisibile.</i>
    ///
    /// Cambia lo stato in <i>EnumGuiState::WaitingForDelay</i>, in cui è memorizzato, assieme all'informazione
    /// <i>ScreenshotDim</i> il <i>JoinHandle</i> del thread.
    fn start_wait_delay(
        &mut self,
        d: f64,
        area: ScreenshotDim,
        frame: &mut eframe::Frame,
        ctx: &eframe::egui::Context,
    ) {
        let jh = std::thread::spawn(move || {
            let duration = if d > 0.0 {
                Duration::from_secs_f64(d)
            } else {
                super::itc::get_animations_delay()
            };
            std::thread::sleep(duration);
        });
        frame.set_visible(false);
        ctx.request_repaint();
        self.state = EnumGuiState::WaitingForDelay(Some(jh), area.clone());
    }

    /// Se nello stato corrente è memorizzato un JoinHandle, esegue <i>join()</i>, mettendo di fatto in attesa la gui (che intanto non è visibile)
    /// fino a quando lo sleep eseguito dal thread non è terminato.
    /// Dopo il <i>join()</i>, rende di nuovo visibile l'applicazione.
    ///
    /// Dopo ciò, richiama un metodo diverso a seconda del tipo di screenshot richiesto.
    ///
    /// <h3>Panics:</h3>
    /// Nel caso <i>self.state</i> sia diverso da <i>EnumGuiState::WaitingForDelay</i>.
    /// Nel caso <i>Option<JoinHandle>></i> == None.
    fn wait_delay(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        if let EnumGuiState::WaitingForDelay(opt_jh, area) = &mut self.state {
            let jh = opt_jh.take().unwrap();
            match jh.join() {
                Ok(_) => {
                    frame.set_visible(true);
                    match *area {
                        ScreenshotDim::Fullscreen => {
                            self.switch_to_edit_image(None, ctx, frame);
                        }
                        ScreenshotDim::Rectangle => {
                            self.switch_to_rect_selection(ctx);
                        }
                    }
                }
                _ => {
                    self.alert.borrow_mut().replace("Timer error".to_string());
                    self.switch_to_main_menu(frame);
                    frame.set_visible(true);
                }
            }
        }
    }

    /*--------------RECT SELECTION---------------------------------------- */
    /// Cambia lo stato della macchina a stati in <i>EnumGuiState::LoadingRectSelection</i>.<br>
    /// Lancia un thread worker per produrre lo screenshot che verrà ritagliato da RectSelection, memorizzando
    /// l'estremità <i>Receiver</i> del canale di comunicazione con tale thread nello stato corrente.
    ///
    fn switch_to_rect_selection(&mut self, ctx: &eframe::egui::Context) {
        if DEBUG {
            println!("nframe (switch to rect selection): {}", ctx.frame_nr());
        }
        self.state = EnumGuiState::LoadingRectSelection(
            self.screens_manager.start_thread_fullscreen_screenshot(),
        );
    }

    /// Esegue <i>Receiver::try_recv()</i> per controllare se il thread worker ha prodotto lo screenshot:
    /// - Se il canale contiene <i>Ok(RgbaImage)</i>, cambia lo stato corrente in <i>EnumGuiState::RectSelection</i>;
    /// - Se il canale contiene <i>Err(&'static str)</i> o se il canale è stato chiuso, scrive l'errore nello stato
    ///     globale dell'applicazione;
    /// - Se il canale è ancora vuoto, richiede un nuovo refresh della gui.
    ///
    /// <i>NOTA: Si è scelta una soluzione con busy wait per evitare che il main thread rimanga in attesa bloccante di
    /// un altro thread, la cui computazione può potenzialmente fallire.</i>
    ///
    /// <h3>Panics:</h3>
    /// Nel caso <i>self.state</i> sia diverso da <i>EnumGuiState::LoadingRectSelection</i>.
    fn load_rect_selection(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        match &mut self.state {
            EnumGuiState::LoadingRectSelection(r) => match r.try_recv() {
                Ok(msg) => {
                    frame.set_visible(true);
                    frame.set_fullscreen(true);
                    match msg {
                        Ok(img) => {
                            let rs = RectSelection::new(img, ctx);
                            self.state = EnumGuiState::RectSelection(rs);
                        }
                        Err(error_message) => {
                            self.alert
                                .borrow_mut()
                                .replace("An error occurred. Impossible to continue.".to_string());
                            let _ = writeln!(std::io::stderr(), "Error: {}", error_message);
                        }
                    }
                }

                Err(TryRecvError::Disconnected) => {
                    frame.set_visible(true);
                    self.alert.borrow_mut().replace(
                        "An error occurred when trying to start the service. Please retry."
                            .to_string(),
                    );
                    self.switch_to_main_menu(frame);
                }
                Err(TryRecvError::Empty) => {
                    frame.set_visible(false); // necessario per la scomparsa della finestra
                    ctx.request_repaint();
                }
            },

            _ => unreachable!(),
        }
    }

    /// Richiama <i>RectSelection::update</i> e ne gestisce il valore di ritorno nel caso questo sia <i>Some((Rect, RgbaImage))</i>,
    /// passando i due parametri al metodo <i>Self::switch_to_edit_image()</i>.
    ///  
    /// <h3>Panics:</h3>
    /// Nel caso <i>self.state</i> sia diverso da <i>EnumGuiState::RectSelection</i>.
    fn show_rect_selection(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        if let EnumGuiState::RectSelection(ref mut rs) = self.state {
            ctx.request_repaint(); //per evitare il bug durante la transizione
            if let Some((rect, rgba)) = rs.update(ctx) {
                frame.set_visible(true);
                self.switch_to_edit_image(Some((rect, rgba)), ctx, frame);
            }
        } else {
            unreachable!();
        }
    }

    /*---------------------------EDIT IMAGE---------------------------------------------------- */

    /// A seconda del valore del parametro <b>opt_rect_img</b> si comporta in modo diverso:
    /// - <i>Some((Rect, RgbaImage))</i>:
    ///     avvia il thread che esegue il ritaglio dell'immagine, salva l'estremità ricevente del canale di comunicazione con
    ///     il thread all'interno dello stato;
    /// - <i>None</i>:
    ///     avvia il thread che esegue lo screenshot fullscreen, salva l'estremità ricevente del canale di comunicazione con
    ///     il thread all'interno dello stato.
    /// In entrambi i casi, il prossimo stato della macchina a stati sarà <i>EnumGuiState::LoadingEditImage.
    fn switch_to_edit_image(
        &mut self,
        opt_rect_img: Option<(Rect, RgbaImage)>,
        ctx: &eframe::egui::Context,
        frame: &mut eframe::Frame,
    ) {
        if let Some((rect, img)) = opt_rect_img {
            self.state =
                EnumGuiState::LoadingEditImage(image_coding::start_thread_crop_image(rect, img));
        } else {
            frame.set_visible(false);
            ctx.request_repaint();
            self.state = EnumGuiState::LoadingEditImage(
                self.screens_manager.start_thread_fullscreen_screenshot(),
            );
        }
    }

    /// Richiama <i>Receiver::try_recv()</i> sul receiver memorizzato nello stato corrente:
    /// - Se la <i>recv()</i> ha successo:
    ///     1. avvia il thread per copiare nella clipboard l'immagine ricevuta tramite il canale;
    ///     2. richiama EditImage::new(), a cui passa l'immagine ricevuta tramite il canale;
    ///     3. cambia lo stato corrente in <i>EnumGuiState::EditImage</i>, in cui memorizza a nuova istanza di <i>EditImage</i>.
    /// - Se il canale è vuoto, mostra uno spinner;
    /// - Se il canale è stato chiuso inaspettatamente, scrive un messaggio di errore nello stato di errore globale.
    ///
    /// <h3>Panics:</h3>
    /// Nel caso <i>self.state</i> sia diverso da <i>EnumGuiState::LoadingEditImage</i>.
    fn load_edit_image(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        if let EnumGuiState::LoadingEditImage(r) = &mut self.state
        //attesa dell'immagine da caricare
        {
            match r.try_recv() {
                Ok(Ok(img)) => {
                    if self.save_settings.borrow().copy_on_clipboard {
                        self.clipboard = Some(start_thread_copy_to_clipboard(&img));
                    }

                    let em = EditImage::new(img, ctx);
                    frame.set_fullscreen(false);
                    frame.set_visible(true);
                    self.state = EnumGuiState::EditImage(em);
                }
                Err(TryRecvError::Empty) => {
                    show_loading(ctx);
                }
                Err(TryRecvError::Disconnected) | Ok(Err(_)) => {
                    self.alert
                        .borrow_mut()
                        .replace("Unable to load the image. please retry".to_string());
                    self.switch_to_main_menu(frame);
                }
            }
        } else {
            unreachable!();
        }
    }

    /// Richiama <i>EditImage::update()</i> e ne gestisce il valore di ritorno:
    /// - <i>EditImageEvent::Saved</i>: avvia la procedura di salvataggio dell'immagine ritornata dal metodo (che è quindi uno screenshot, con eventuali
    ///     annotazioni) nel formato corrispondente all'oggetto <i>ImageFormat</i> ritornato dal metodo;
    /// - <i>EditImageEvent::Aborted</i>: ritorna alla schermata principale eliminando tutti i progressi;
    /// - <i>EditImageEvent::Nil</i>: non è necessaria alcuna azione.
    ///
    /// <h3>Panics:</h3>
    /// Nel caso <i>self.state</i> sia diverso da <i>EnumGuiState::EditImage</i>.
    fn show_edit_image(
        &mut self,
        ctx: &eframe::egui::Context,
        frame: &mut eframe::Frame,
        enabled: bool,
    ) {
        if let EnumGuiState::EditImage(em) = &mut self.state {
            match em.update(ctx, enabled) {
                FrameEvent::Saved { image, format } => {
                    self.manage_save_request(image, format);
                }
                FrameEvent::Aborted => self.switch_to_main_menu(frame),
                FrameEvent::Nil => (),
            }
        } else {
            unreachable!();
        }
    }

    ///1. Lancia un thread che, consultando le <i>save_settings</i> dell'applicazione ed eventualmente
    /// mostrando un file dialog, ottiene il path del file di salvataggio dell'immagine;
    /// 2. salva in <i>GlobalGuiState</i>il <i>Receiver</i> del canale di comunicazione con il thread lanciato
    /// precedentemente. La presenza di tale <i>Receiver</i> nello stato globale causerà la disabilitazione
    /// dell'altra finestra attualmente mostrata.
    fn manage_save_request(&mut self, image: RgbaImage, format: ImageFormat) {
        let rx = self.save_settings.borrow().compose_output_file_path(format);
        self.pending_save_request = Some((rx, image));
    }

    ///In seguito alla creazione di una richiesta di salvataggio, gestisce l'attesa (ripetendo
    /// la chiamata di <i>try_recv()</i>) che il thread demandato a gestire il file dialog
    /// invii sul canale.
    /// Quando la option viene ricevuta sul canale:
    /// - se è Some(path), avvia il thread che salverà l'immagine presso il path ricevuto e cambia
    ///     lo stato della gui in <i>EnumGuiState::Saving</i>;
    /// - se è None, elimina la richiesta di salvataggio.
    /// Se il canale si è chiuso, segnala l'errore.
    fn wait_output_file_path(&mut self) {
        if let Some((rx, _)) = &self.pending_save_request {
            match rx.try_recv() {
                //L'utente non ha annullato il salvataggio e il path di output è disponibile:
                Ok(Some(pb)) => {
                    if let Some((_, img)) = self.pending_save_request.take() {
                        self.state =
                            EnumGuiState::Saving(image_coding::start_thread_save_image(pb, img));
                    }
                }
                //L'utente ha annullato il salvataggio => viene eliminata la richiesta pending:
                Ok(None) => {
                    let _ = self.pending_save_request.take();
                }
                //L'operazione è ancora in corso:
                Err(TryRecvError::Empty) => (),
                //Si è verificato un errore e il canale di comunicazione con il thread è stato chiuso:
                Err(TryRecvError::Disconnected) => {
                    let _ = self.pending_save_request.take();
                    self.alert
                        .borrow_mut()
                        .replace("Error: image not saved".to_string());
                }
            }
        }
    }

    //----------------------SAVING --------------------------------------------------
    /// Esegue busy waiting sul canale di comunicazione con il thread worker iterando la chiamata al metodo <i>Receiver::try_recv()</i>:
    /// - Fino a quando non compare un messaggio nel canale mostra uno spinner;
    /// - Se il canale viene chiuso inaspettatamente o se nel canale compare un oggetto <i>Err()</i>, scrive un messaggio nello stato di
    ///     errore globale dell'applicazione;
    /// - Se nel canale compare un oggetto Ok(), mostra un alert con un messaggio di conferma e riporta l'applicazione nella schermata
    ///     di partenza.
    ///
    /// <h3>Panics:</h3>
    /// Nel caso in cui <i>self.state</i> sia diverso da <i>EnumGuiState::Saving</i>.
    fn show_saving(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        if let EnumGuiState::Saving(rx) = &mut self.state {
            match rx.try_recv() {
                Ok(Ok(path)) => {
                    let mut msg = String::from("Image saved: \n\n");
                    msg.push_str(&path);
                    self.alert.borrow_mut().replace(msg);
                    self.switch_to_main_menu(frame);
                }
                Err(TryRecvError::Empty) => show_loading(ctx),
                Err(TryRecvError::Disconnected) | Ok(Err(_)) => {
                    self.alert
                        .borrow_mut()
                        .replace("Error: image not saved".to_string());
                    self.switch_to_main_menu(frame);
                }
            }
        } else {
            unreachable!();
        }
    }

    /// Esegue l'azione relativa alla hotkey <b>hn</b>.
    fn hotkey_reaction(
        &mut self,
        hn: HotkeyName,
        ctx: &eframe::egui::Context,
        frame: &mut eframe::Frame,
    ) {
        if DEBUG {
            println!("hotkey_reaction");
        }
        frame.focus();
        match hn {
            HotkeyName::FullscreenScreenshot => self.switch_to_edit_image(None, ctx, frame),
            HotkeyName::RectScreenshot => self.switch_to_rect_selection(ctx),
        }
    }

    /// Esegue busy waiting sul canale di comunicazione con il thread worker che sta copiando l'immagine nella clipboard.<br>
    /// Gestisce la ricezione sul canale sia di un messaggio di conferma che di un messaggio di errore, comunicando all'user
    /// l'esito dell'operazione.
    /// Mostra errore nel caso il canale venga chiuso inaspettatamente.
    fn manage_clipboard(&mut self) {
        if let Some(rx) = &self.clipboard {
            match rx.try_recv() {
                Ok(Ok(_)) => {
                    self.clipboard = None;
                }
                Ok(Err(e)) => {
                    self.alert.borrow_mut().replace(format!(
                        "Error: impossible to copy the image on the clipboard ({})",
                        e
                    ));
                    self.clipboard = None;
                }
                Err(TryRecvError::Disconnected) => {
                    self.alert.borrow_mut().replace(
                        "Error: impossible to copy the image on the clipboard".to_string(),
                    );
                    self.clipboard = None;
                }
                Err(TryRecvError::Empty) => (),
            }
        }
    }
}

pub fn launch_gui() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Simple screenshot App",
        options,
        Box::new(|_cc| Box::new(GlobalGuiState::new())),
    )
    .unwrap();
}

impl eframe::App for GlobalGuiState {
    /// Attiva di default l'ascolto della pressione delle hotkeys: potrà essere eventualmente disattivato dai metodi che verranno
    /// richiamati successivamente da questo metodo. Si è scelto questo approccio perché sono poche le casistiche in cui l'ascolto
    /// debba essere disattivato.<br>
    /// Controlla se ci sono eventuali thread worker che stanno facendo operazioni sulla clipboard da gestire.<br>
    /// A seconda dello stato corrente (una delle varianti di <i>EnumGlobalGuiState</i>) esegue una diversa operazione (eseguendo un match case).<br>
    /// Solo se attualmente non è mostrato nessun alert, controlla se nell'input di questo frame c'è la pressione di una hotkey:
    /// in caso positivo, la gestisce.
    /// Se invece lo stato di errore globale non è vuoto, mostra un alert con il messaggio che descrive tale errore.
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        //if crate::DEBUG {print!("gui refresh. ");}

        let main_window_enabled = self.alert.borrow().is_none()
            && self.pending_save_request.is_none()
            && self.directory_dialog_receiver.is_none();

        //se non è ancora stato fatto partire il thread che ascolta le hotkey, si crea un canale di comunicazione e si richiama l'apposita funzione del modulo hotkeys
        if self.hotkey_receiver.is_none() {
            let (tx, rx) = channel();
            self.hotkey_receiver = Some(rx);
            hotkeys::start_thread_listen_hotkeys(
                Arc::new(ctx.clone()),
                self.registered_hotkeys.clone(),
                tx,
            );
        }

        self.registered_hotkeys.set_listen_enabled(true); //abilito di default l'ascolto delle hotkeys (potrà essere disabilitato dalle funzioni chiamate nei rami del match)

        //gestione di eventuali operazioni sulla clipboard
        self.manage_clipboard();

        /*
        if crate::DEBUG {
            println!("state = {:?}", self.state);
        }
        */

        match &mut self.state {
            EnumGuiState::MainMenu(..) => {
                self.show_main_menu(ctx, frame, main_window_enabled);
            }
            EnumGuiState::WaitingForDelay(..) => {
                self.wait_delay(ctx, frame);
            }
            EnumGuiState::LoadingRectSelection(..) => {
                self.load_rect_selection(ctx, frame);
            }
            EnumGuiState::RectSelection(..) => {
                self.show_rect_selection(ctx, frame);
            }
            EnumGuiState::LoadingEditImage(..) => {
                self.load_edit_image(ctx, frame);
            }
            EnumGuiState::EditImage(..) => {
                self.show_edit_image(ctx, frame, main_window_enabled);
            }
            EnumGuiState::Saving(..) => {
                self.show_saving(ctx, frame);
            }
        }

        //ascolto di hotkeys solo nel caso non sia in corso il display di un messaggio di errore
        if main_window_enabled {
            if let Some(hr) = &self.hotkey_receiver {
                match hr.try_recv() {
                    Ok(hn) => self.hotkey_reaction(hn, ctx, frame),
                    Err(TryRecvError::Empty) => (),
                    Err(TryRecvError::Disconnected) => {
                        self.alert
                            .borrow_mut()
                            .replace("Error in hotkeys. Temporarily unavailable".to_string());
                        self.hotkey_receiver = None;
                    }
                }
            }
        } else {
            //segnalazione eventuali errori
            if self.alert.borrow().is_some() {
                error_alert::show_error_alert(ctx, &mut self.alert.borrow_mut())
            }
            //attesa del risultato del file dialog
            if self.pending_save_request.is_some() {
                self.wait_output_file_path();
                ctx.request_repaint();
            } else if self.directory_dialog_receiver.is_some() {
                self.wait_directory_dialog();
                ctx.request_repaint();
            }
        }

        //ctx.request_repaint();
    }
}
