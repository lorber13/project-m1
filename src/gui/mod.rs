/*
La gui, a causa delle limitazioni imposte da eframe, deve essere eseguta solo nel thread pricipale.
Questo modulo è disegnato per permettere al thread che esegue la gui di rimanere sempre in esecuzione,
mostrando, a seconda delle necessità, una diversa finestra tra quelle elencate nella enum EnumGuiState.

La gui è quindi intesa come macchina a stati e le varianti della EnumGuiState incapsulano le variabili con i dettagli di ciascuno stato. 
In particolare, se una variante incapsula un Receiver, allora la gui è in uno stato di attesa: viene fatto busy waiting con tryRecv(). Si noti che il design della sincronizzazione con altri thread, appena descritto, non aggiunge overhead perchè asseconda il funzionamento del crate eframe.

Lo stato della gui è incapsulato dentro la struct <i>GlobalGuiState</i> assieme ad altre informazioni globali.
 */


mod capture_mode;
mod rect_selection;
mod error_alert;
pub mod file_dialog;
mod loading;
mod edit_image;
mod save_settings;
mod hotkeys_settings;
mod menu;

use rect_selection::RectSelection;
use std::cell::RefCell;
use std::fmt::Formatter;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use eframe::egui::Rect;
use image::{RgbaImage, ImageError};
use crate::itc::ScreenshotDim;
use crate::{DEBUG, image_coding, screens_manager};
use crate::gui::loading::show_loading;
use crate::image_coding::{start_thread_copy_to_clipboard, ImageFormat};
use edit_image::EditImage;
use self::edit_image::EditImageEvent;
use self::menu::MainMenuEvent;
use save_settings::SaveSettings;
use menu::MainMenu;
use crate::hotkeys::{RegisteredHotkeys, HotkeyName};
use std::io::Write;
use std::rc::Rc;

/// Possibili valori dello stato interno della macchina a stati realizzata dalla struct <i>GlobalGuiState</i>.
pub enum EnumGuiState
{
    MainMenu(MainMenu),
    WaitingForDelay(Option<JoinHandle<()>>,ScreenshotDim),
    LoadingRectSelection(Receiver<Result<RgbaImage, &'static str>>),
    RectSelection(RectSelection),
    LoadingEditImage(Receiver<Result<RgbaImage, &'static str>>),
    EditImage(EditImage),
    Saving(Receiver<Result<(), ImageError>>)
}

impl std::fmt::Debug for EnumGuiState
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error>
    {
        match self
        {
            EnumGuiState::MainMenu(_) => write!(f, "EnumGuiState::MainMenu"),
            EnumGuiState::WaitingForDelay(..) => write!(f, "EnumGuiState::WaitingForDelay"),
            EnumGuiState::LoadingRectSelection(..) => write!(f, "EnumGuiState::LoadingRectSelection"),
            EnumGuiState::RectSelection(..) => write!(f, "EnumGuiState::RectSelection"),
            EnumGuiState::EditImage(..) => write!(f, "EnumGuiState::EditImage"),
            EnumGuiState::LoadingEditImage(_) => write!(f, "EnumGuiState::LoadingEdiImage"),
            EnumGuiState::Saving(_) => write!(f, "EnumGuiState::Saving")
        }
    }
}

/// Memorizza lo stato globale della dell'applicazione.
pub struct GlobalGuiState
{
    /// Stato corrente della macchina a stati (quindi, dell'intera applicazione).
    state: EnumGuiState,
    /// Stato di errore globale dell'applicazione.
    alert: Rc<RefCell<Option<String>>>,
    /// Gestore degli schermi rilevati dal sistema.
    screens_manager: Arc<screens_manager::ScreensManager>,
    /// Impostazioni di salvataggio automatico delle immagini.
    save_settings: Rc<RefCell<SaveSettings>>,
    /// Gestore delle hotkeys registrate e del loro ascolto.
    registered_hotkeys: Arc<RegisteredHotkeys>,
    /// Contiene Some() se è stato lanciato un worker per copiare dati sulla clipboard.
    clipboard : Option<Receiver<Result<(), arboard::Error>>>, 
}



impl GlobalGuiState
{
    /// Crea una nuova istanza della macchina a stati, del gestore delle hotkeys e degli schermi.
    /// Lo stato iniziale è <i>EnumGuiState::MainMenu</i>.
    fn new() -> Self
    {
        let alert: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let registered_hotkeys = RegisteredHotkeys::new();
        let save_settings = Rc::new(RefCell::new(SaveSettings::new(alert.clone())));
        let screens_manager = screens_manager::ScreensManager::new(150);
        GlobalGuiState {
            state: EnumGuiState::MainMenu(MainMenu::new(alert.clone(), screens_manager.clone(), save_settings.clone(), registered_hotkeys.clone())),
            alert,
            screens_manager,
            save_settings,
            registered_hotkeys,
            clipboard: None
        }
    }


    /// Modifica lo stato della macchina a stati in <i>EnumGuistate::MainMenu</i>, in cui memorizza una nuova istanza di MainMenu.
    fn switch_to_main_menu(&mut self, _frame: &mut eframe::Frame)
    {
        _frame.set_decorations(true);
        _frame.set_fullscreen(false);
        _frame.set_maximized(false);
        _frame.set_window_size(eframe::egui::Vec2::new(500.0, 300.0));
        _frame.set_visible(true);
        self.state = EnumGuiState::MainMenu(MainMenu::new(self.alert.clone(), self.screens_manager.clone(), self.save_settings.clone(), self.registered_hotkeys.clone()));
    }

    /// Esegue il metodo <i>MainMenu::update()</i>, a cui passa enabled = false solo se l'applicazione sta mostrando la finestra di alert.
    /// In questo modo, fino a quando la finestra di alert non verrà chiusa, i click eseguiti dall'user su MainMenu non avranno effetto.<br>
    /// Gestisce inoltre il caso in cui <i>MainMenu::update()</i> restituisca <i>MainMenuEvent::ScreenshotRequest</i>, richiamando
    /// <i>Self::start_wait_delay()</i>.
    ///  
    /// <h3>Panics:</h3>
    /// Nel caso <i>self.state</i> sia diverso da <i>EnumGuiState::MainMenu</i>.
    fn show_main_menu(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame)
    {
        if let EnumGuiState::MainMenu(m) = &mut self.state
        {
            let enabled = self.alert.borrow().is_none();
            match m.update(enabled, ctx, frame)
            {
                MainMenuEvent::ScreenshotRequest(sd, d ) => self.start_wait_delay(d, sd, frame, ctx), 
                MainMenuEvent::Nil => ()
            }
        }else {unreachable!();}
    }

    /// Data una richiesta di screenshot, se essa include un delay non nullo, rende invisibile l'applicazione e 
    /// lancia il thread che esegue la sleep relativa.
    /// 
    /// 
    /// Cambia lo stato in <i>EnumGuiState::WaitingForDelay</i>, in cui è memorizzato, assieme all'informazione
    /// <i>ScreenshotDim</i> una option contenente:
    /// - None, se il delay associato alla richiesta di screenshot era nullo;
    /// - altrimenti, il JoinHandle relativo al thread appena lanciato.
    fn start_wait_delay(&mut self, d: f64, area: ScreenshotDim, frame: &mut eframe::Frame,ctx: &eframe::egui::Context) {
        let mut jh=None;
        if d > 0.0
        {
            frame.set_visible(false);
            ctx.request_repaint();
            jh = Some(std::thread::spawn(move||{
                thread::sleep(Duration::from_secs_f64(d));
            }));
        }
        self.state = EnumGuiState::WaitingForDelay(jh, area.clone());
    }


    /// Se nello stato corrente è memorizzato un JoinHandle, esegue <i>join()</i>, mettendo di fatto in attesa la gui (che intanto non è visibile) 
    /// fino a quando lo sleep eseguito dal thread non è terminato. Dopo il <i>join()</i>, rende dinuovo visibile l'applicazione.
    /// 
    /// Dopo ciò, richiama un metodo diverso a seconda del tipo di screenshot richiesto:
    /// - <i>ScreenshotDim::FullScreen</i>
    /// 
    /// <h3>Panics:</h3>
    /// Nel caso <i>self.state</i> sia diverso da <i>EnumGuiState::WaitingForDelay</i>.
    fn wait_delay(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame)
    {
        if let EnumGuiState::WaitingForDelay(opt_jh, area)=&mut self.state
        {
            let area_clone = area.clone();
            let temp=opt_jh.take();
            if let Some(jh)=temp{
                match jh.join() {
                    Ok(_) => {
                        frame.set_visible(true);
                    },
                    _ => {
                        self.alert.borrow_mut().replace("Timer error".to_string());
                        self.switch_to_main_menu(frame);
                    }
                }
            }
            match area_clone {
                ScreenshotDim::Fullscreen => {
                    self.switch_to_edit_image(None, ctx, frame);
                }
                ScreenshotDim::Rectangle => {
                    self.switch_to_rect_selection(ctx);
                }
            }
        }
    }


    /*--------------RECT SELECTION---------------------------------------- */
    /// Cambia lo stato della macchina a stati in <i>EnumGuiState::LoadingRectSelection</i>.<br>
    /// Lancia un thread worker per produrre lo screenshot che verrà ritagliato da RectSelection, memorizzando 
    /// l'estremità <i>Receiver</i> del canale di comunicazione con tale thread nello stato corrente.
    /// 
    fn switch_to_rect_selection(&mut self, ctx: &eframe::egui::Context)
    {
        if DEBUG { println!("nframe (switch to rect selection): {}", ctx.frame_nr()); }
        self.state = EnumGuiState::LoadingRectSelection(self.screens_manager.start_thread_fullscreen_screenshot());
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
    fn load_rect_selection(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame)
    {
        match &mut self.state
        {
            EnumGuiState::LoadingRectSelection(r) => 
            {
                match r.try_recv()
                {
                    Ok(msg) =>
                    {
                        ctx.request_repaint();
                        frame.set_visible(true);
                        frame.set_fullscreen(true);
                        match msg {
                            Ok(img) => {
                                let rs = RectSelection::new(img, ctx);
                                self.state = EnumGuiState::RectSelection(rs);
                            }
                            Err(error_message) => {
                                self.alert.borrow_mut().replace("An error occoured. Impossible to continue.".to_string());
                                let _ = writeln!(std::io::stderr(), "Error: {}", error_message);
                            }
                        }
                    },

                    Err(TryRecvError::Disconnected) => {
                        frame.set_visible(true);
                        self.alert.borrow_mut().replace("An error occoured when trying to start the service. Please retry.".to_string());
                        self.switch_to_main_menu(frame);
                    },
                    Err(TryRecvError::Empty) => ctx.request_repaint()
                }
            },

            _ => unreachable!()
        }
        
        
    }

    /// Richiama <i>RectSelection::update</i> e ne gestisce il valore di ritorno nel caso questo sia <i>Some((Rect, RgbaImage))</i>,
    /// passando i due parametri al metodo <i>Self::switch_to_edit_image()</i>.
    ///  
    /// <h3>Panics:</h3>
    /// Nel caso <i>self.state</i> sia diverso da <i>EnumGuiState::RectSelection</i>.
    fn show_rect_selection(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame)
    {
        if let EnumGuiState::RectSelection(ref mut rs) = self.state
        {
            if let Some((rect, rgba)) = rs.update(ctx) {
                self.switch_to_edit_image(Some((rect, rgba)), ctx, frame);
            }
        }else {unreachable!();}
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
    fn switch_to_edit_image(&mut self, opt_rect_img: Option<(Rect, RgbaImage)>, ctx: &eframe::egui::Context, frame: &mut eframe::Frame)
    {
        if let Some((rect, img)) = opt_rect_img
        {
            self.state = EnumGuiState::LoadingEditImage(image_coding::start_thread_crop_image(rect, img));
        }else
        {
            frame.set_visible(false);
            ctx.request_repaint();
            self.state = EnumGuiState::LoadingEditImage(self.screens_manager.start_thread_fullscreen_screenshot());
        }
    }

    /// Richiama <i>Receiver::try_recv()</i> sul receiver memorizzato nello stato corrente:
    /// - Se la <i>recv()</i> ha successo:
    ///     1. avvia il thread per copiare nella clipboard l'immagine ricevuta tramite il canale;
    ///     2. richiama EditImage::new(), a cui passa l'immagine ricevuta tramite il canale;
    ///     3. cambia lo stato corrente in <i>EnumGuistate::EditImage</i>, in cui memorizza a nuova istanza di <i>EditImage</i>.
    /// - Se il canale è vuoto, mostra uno spinner;
    /// - Se il canale è stato chiuso inaspettatamente, scrive un messaggio di errore nello stato di errore globale.
    /// 
    /// <h3>Panics:</h3>
    /// Nel caso <i>self.state</i> sia diverso da <i>EnumGuiState::LoadingEditImage</i>.
    fn load_edit_image(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame)
    {
        if let EnumGuiState::LoadingEditImage(r) = &mut self.state //attesa dell'immagine da caricare
        {
            match r.try_recv()
            {
                Ok(Ok(img)) => {
                    
                    if self.save_settings.borrow().copy_on_clipboard {self.clipboard =Some(start_thread_copy_to_clipboard(&img));}

                    let em = EditImage::new(img, ctx);
                    frame.set_fullscreen(false);
                    frame.set_visible(true);
                    self.state = EnumGuiState::EditImage(em);
                }
                Err(TryRecvError::Empty) => {show_loading(ctx);},
                Err(TryRecvError::Disconnected) | Ok(Err(_)) => {self.alert.borrow_mut().replace("Unable to load the image. please retry".to_string()); self.switch_to_main_menu(frame);}
            }
        }else {unreachable!();}
    }


    /// Richiama <i>EditImage::update()</i> e ne gestisce il valore di ritorno:
    /// - <i>EditImageEvent::Saved</i>: avvia la procedura di salvataggio dell'immagine ritornata dal metodo (che è quindi uno screenshot, con eventuali
    ///     annotazioni) nel formato corrispondente all'oggetto <i>ImageFormat</i> ritornato dal metodo;
    /// - <i>EditImageEvent::Aborted</i>: ritorna alla schermata principale eliminando tutti i progressi;
    /// - <i>EditImageEvent::Nil</i>: non è necessaria alcuna azione.
    /// 
    /// <h3>Panics:</h3>
    /// Nel caso <i>self.state</i> sia diverso da <i>EnumGuiState::EditImage</i>.
    fn show_edit_image(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame)
    {
        if let EnumGuiState::EditImage(em) = &mut self.state
        {
            let enabled = self.alert.borrow().is_none();
            match em.update(ctx, frame, enabled)
            {
                EditImageEvent::Saved {image, format} => 
                {
                    self.manage_save_request(image, format);
                },
                EditImageEvent::Aborted => { self.switch_to_main_menu(frame)},
                EditImageEvent::Nil => ()
            }
               
        }else {unreachable!();}
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
    fn manage_save_request(&mut self, image: RgbaImage, format: ImageFormat)
    {
        let ss = self.save_settings.borrow().clone();
        match (ss.get_default_dir(), ss.get_default_name())
        {
            (Some(dp), Some(dn)) => 
            {
                let pb = PathBuf::from(dp);
                let ext: &str = format.into();
                self.state = EnumGuiState::Saving(image_coding::start_thread_save_image(pb, dn,String::from(ext), image ));
            }

            (None, Some(dn)) =>
            {
                let dir_opt = file_dialog::show_directory_dialog(None);
                if let Some(dir) = dir_opt
                {
                    if DEBUG {let _ =writeln!(std::io::stdout(), "DEBUG: dir picker return = {}", dir.display());}

                    let ext: &str = format.into();
                    self.state = EnumGuiState::Saving(image_coding::start_thread_save_image(dir, dn,String::from(ext), image ));
                }
            },

            (Some(dp), None) =>
            {
                let dir_opt = file_dialog::show_save_dialog(&format, Some(&dp.to_string()));
                if let Some(dir) = dir_opt
                {
                    let ext: &str = format.into();
                    let file_name = String::from(dir.file_name().unwrap().to_str().unwrap());
                    self.state = EnumGuiState::Saving(image_coding::start_thread_save_image(dir, file_name,String::from(ext), image ));
                }
            },

            (None, None) =>
            {
                let dir_opt = file_dialog::show_save_dialog(&format, None);
                if let Some(dir) = dir_opt
                {
                    let ext: &str = format.into();
                    let file_name = String::from(dir.file_name().unwrap().to_str().unwrap());
                    self.state = EnumGuiState::Saving(image_coding::start_thread_save_image(dir, file_name,String::from(ext), image ));
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
    fn show_saving(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame)
    {
        if let EnumGuiState::Saving(rx) = &mut self.state
        {
            match rx.try_recv()
            {
                Ok(Ok(_)) =>
                {
                    self.alert.borrow_mut().replace("Image saved!".to_string());
                    self.switch_to_main_menu(frame);
                },
                Err (TryRecvError::Empty) => show_loading(ctx),
                Err(TryRecvError::Disconnected) | Ok(Err(_)) => {self.alert.borrow_mut().replace("Error: image not saved".to_string()); self.switch_to_main_menu(frame);}
            }
        }else {unreachable!();}
    }

    /// Esegue l'azione relativa alla hotkey <b>hn</b>.
    fn hotkey_reaction(&mut self, hn: HotkeyName, ctx: &eframe::egui::Context, frame: &mut eframe::Frame)
    {
        match hn
        {
            HotkeyName::FullscreenScreenshot => self.switch_to_edit_image(None, ctx, frame),
            HotkeyName::RectScreenshot => self.switch_to_rect_selection(ctx)
        }
    }

    /// Esegue busy waiting sul canale di comunicazione con il thread worker che sta copiando l'immagine nella clipboard.<br>
    /// Gestisce la ricezione sul canale sia di un messaggio di conferma che di un messaggio di errore, comunicando all'user
    /// l'esito dell'operazione.
    /// Mostra errore nel caso il canale venga chiuso inaspettatamente.
    fn manage_clipboard(&mut self)
    {
        if let Some(rx) = &self.clipboard
        {
            match rx.try_recv()
            {
                Ok(Ok(_)) =>{ self.clipboard = None; },
                Ok(Err(e)) => {self.alert.borrow_mut().replace(format!("Error: impossible to copy the image on the clipboard ({})", e)); self.clipboard = None;}
                Err(TryRecvError::Disconnected) => {self.alert.borrow_mut().replace("Error: impossible to copy the image on the clipboard".to_string()); self.clipboard = None;},
                Err(TryRecvError::Empty) => ()
            }
        }
    }
    
}



pub fn launch_gui()
{  
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Simple screenshot App", 
        options,  
        Box::new(|_cc| { return Box::new(GlobalGuiState::new()); })
    ).unwrap();
}


impl eframe::App for GlobalGuiState
{
    /// Attiva di default l'ascolto della pressione delle hotkeys: potrà essere eventualmente disattivato dai metodi che verranno 
    /// richiamati successivamente da questo metodo. Si è scelto questo approccio perchè sono poche le casistiche in cui l'ascolto 
    /// debba essere disattivato.<br>
    /// Controlla se ci sono eventuali thread worker che stanno facendo operazioni sulla clipboard da gestire.<br>
    /// A seconda dello stato corrente (una delle varianti di <i>EnumGlobalGuiState</i>) esegue una diversa operazione (eseguendo un match case).<br>
    /// Solo se attualmente non è mostrato nessun alert, controlla se nell'input di questo frame c'è la pressione di una hotkey:
    /// in caso positivo, la gestisce.
    /// Se invece lo stato di errore globale non è vuoto, mostra un alert con il messaggio che descrive tale errore.
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) 
    {
        //if crate::DEBUG {print!("gui refresh. ");}

        self.registered_hotkeys.set_listen_enabled(true); //abilito di default l'ascolto delle hotkeys (potrà essere disabilitato dalle funzioni chiamate nei rami del match)

        //gestione di eventuali operazioni sulla clipboard
        self.manage_clipboard();

        //if crate::DEBUG {println!("state = {:?}", self.state);}

        match &mut self.state
        {
            EnumGuiState::MainMenu(..) =>
            {
                self.show_main_menu(ctx, frame);
            },
            EnumGuiState::WaitingForDelay(..) =>
            {
                self.wait_delay(ctx, frame);
            },
            EnumGuiState::LoadingRectSelection(..) =>
            {
                self.load_rect_selection(ctx, frame);
            },
            EnumGuiState::RectSelection(..) => {
                    self.show_rect_selection(ctx, frame);
            }, 
            EnumGuiState::LoadingEditImage(..) =>
            {
                self.load_edit_image(ctx, frame);
            },
            EnumGuiState::EditImage(..) =>
                {
                    self.show_edit_image(ctx, frame);
                },
            EnumGuiState::Saving(..) =>
            {
                self.show_saving(ctx, frame);
            }
            
        }

        //ascolto di hotkeys
        if self.alert.borrow().is_none()
        {
            match self.registered_hotkeys.listen_hotkeys()
            {
                None => (),
                Some(hn) => self.hotkey_reaction(hn, ctx, frame)
            }
        }else 
        {
            //segnalazione eventuali errori
            error_alert::show_error_alert(ctx, &mut self.alert.borrow_mut());
        }
         
    }
}
