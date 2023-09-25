c/*
La gui, a causa delle limitazioni imposte da eframe, deve essere eseguta solo nel thread pricipale.
Questo modulo è disegnato per permettere al thread che esegue la gui di rimanere sempre in esecuzione,
mostrando, a seconda delle necessità, una diversa finestra tra quelle elencate nella enum EnumGuiState (inclusa None).
Il modulo offre un'interfaccia piu' esterna (Gui, che è un façade) che offre i metodi per passare da
una finestra all'altra.
Il  modulo memorizza internamente (nella classe GlobalGuiState) un Sender<SignalToHeadThread> per inviare
segnali al thread che implementa la logica applicativa. E' infatti lo stesso thread che può richiamare
le funzioni pubbliche di Gui per modificare ciò che si vede. 
 */


mod main_window;
mod rect_selection;
mod error_alert;
pub mod file_dialog;
mod loading;
mod edit_image;
mod save_settings;
mod hotkeys_settings;
mod menu;

use rect_selection::RectSelection;
use std::fmt::Formatter;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
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
use self::hotkeys_settings::HotkeysSettings;
use self::menu::MainMenuEvent;
use save_settings::SaveSettings;
use menu::MainMenu;

pub enum EnumGuiState
{
    MainMenu(MainMenu),
    WaitingForDelay(Option<JoinHandle<()>>,ScreenshotDim),
    LoadingRectSelection(u64,Option<Receiver<Result<RgbaImage, &'static str>>>),
    RectSelection(RectSelection),
    LoadingEditImage(Option<Receiver<Result<RgbaImage, &'static str>>>),
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
            EnumGuiState::Saving(_) => write!(f, "EnumGuiState::Start")
        }
    }
}

pub struct GlobalGuiState
{
    state: EnumGuiState,
    alert: Option<&'static str>,
    save_request: Option<(RgbaImage, ImageFormat)>,
    screens_manager: Arc<screens_manager::ScreensManager>,
    save_settings: SaveSettings,
    hotkeys_settings: HotkeysSettings
}



impl GlobalGuiState
{
    fn new() -> Self
    {
        GlobalGuiState {
            state: EnumGuiState::MainMenu(MainMenu::new()),
            alert: None,
            save_request: None,
            screens_manager: screens_manager::ScreensManager::new(150),
            save_settings: SaveSettings::new(),
            hotkeys_settings: HotkeysSettings::new()
        }
    }


    fn switch_to_main_menu(&mut self, _frame: &mut eframe::Frame)
    {
        _frame.set_decorations(true);
        _frame.set_fullscreen(false);
        _frame.set_maximized(false);
        _frame.set_window_size(eframe::egui::Vec2::new(500.0, 300.0));
        _frame.set_visible(true);
        self.state = EnumGuiState::MainMenu(MainMenu::new());
    }

    fn show_main_menu(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame)
    {
        if let EnumGuiState::MainMenu(m) = &mut self.state
        {
            match m.update(self.screens_manager.clone(), &self.save_settings, &self.hotkeys_settings, ctx, frame)
            {
                Some(MainMenuEvent::ScreenshotRequest(sd, d )) => self.start_wait_delay(d, sd, frame, ctx), 
                Some(MainMenuEvent::SaveConfiguration(ss)) => self.save_settings = ss,
                Some(MainMenuEvent::HotkeysConfiguration(hs)) => self.hotkeys_settings = hs,
                None => ()
            }
        }else {unreachable!();}
    }

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
                        self.alert.replace("Timer error");
                        self.switch_to_main_menu(frame);
                    }
                }
            }
            match area_clone {
                ScreenshotDim::Fullscreen => {
                    self.switch_to_edit_image(None, ctx, frame);
                }
                ScreenshotDim::Rectangle => {
                    self.switch_to_rect_selection(ctx, frame);
                }
            }
        }
    }


    /*--------------RECT SELECTION---------------------------------------- */

    fn switch_to_rect_selection(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame)
    {
        frame.set_visible(false);
        ctx.request_repaint();
        if DEBUG { println!("nframe (switch to rect selection): {}", ctx.frame_nr()); }
        self.state = EnumGuiState::LoadingRectSelection(ctx.frame_nr(), None);
    }
    fn load_rect_selection(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame)
    {
        match &mut self.state
        {
            EnumGuiState::LoadingRectSelection(nf, None) => //il thread non è ancora stato spawnato
            {
                if (*nf+13) <= ctx.frame_nr()
                {
                    if DEBUG {println!("nframe (load rect selection): {}", ctx.frame_nr());}
                    let rx = self.screens_manager.start_thread_fullscreen_screenshot();
                    self.state = EnumGuiState::LoadingRectSelection(*nf, Some(rx));
                    ctx.request_repaint();
                }else {
                    ctx.request_repaint();
                }
                
            },

            EnumGuiState::LoadingRectSelection(_, Some(r)) => //in attesa che il thread invii l'immmagine
            {
                //se sono in stato di attesa, controllo se il thread worker ha inviato sul canale
                match r.try_recv()
                {
                    //se un messaggio è stato ricevuto, interrompo lo stato di attesa e visualizzo la prossima schermata
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
                                self.alert = Some(error_message)
                            }
                        }
                    },

                    Err(TryRecvError::Disconnected) => {
                        frame.set_visible(true);
                        self.alert.replace("An error occoured when trying to start the service. Please retry.");
                        self.switch_to_main_menu(frame);
                    },
                    Err(TryRecvError::Empty) => ctx.request_repaint()
                }
            },

            _ => unreachable!()
        }
        
        
    }


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

    ///se opt_rect_img == Some(..),
    ///uso il rettangolo per ritagliare l'immagine precedentemente salvata
    ///un thread worker esegue il task, mentre la gui mostrerà la schermata di caricamento
    /// altrimenti,
    /// avvio un thread worker che eseguirà lo screenshot fullscreen
    fn switch_to_edit_image(&mut self, opt_rect_img: Option<(Rect, RgbaImage)>, ctx: &eframe::egui::Context, frame: &mut eframe::Frame)
    {
        if let Some((rect, img)) = opt_rect_img
        {
            let (tx, rx) = channel();
            thread::spawn(move||
                {
                    let crop_img = Ok(image::imageops::crop_imm::<RgbaImage>(&img,
                                                                                rect.left() as u32,
                                                                                rect.top() as u32,
                                                                                rect.width() as u32,
                                                                                rect.height() as u32).to_image());


                    let _ = tx.send(crop_img);
                });
            self.state = EnumGuiState::LoadingEditImage(Some(rx));
        }else
        {
            frame.set_visible(false);
            ctx.request_repaint();
            self.state = EnumGuiState::LoadingEditImage(None);
        }
        
        // passo nello stadio di attesa dell'immagine ritagliata (non sono ancora dentro editImage)
        
    }

    //pub fn switch_to_none(&mut self)
    //{
    //    let mut cv = Arc::new((Condvar::new(), Mutex::new(false)));
    //    let mut guard = self.state.lock().unwrap();
    //    *guard = EnumGuiState::None(cv.clone());
    //    drop(guard);
    //    cv.0.wait_while(cv.1.lock().unwrap(), |sig| !*sig);
    //}


    fn load_edit_image(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame)
    {
        if let EnumGuiState::LoadingEditImage(Some(r)) = &mut self.state //attesa dell'immagine da caricare
        {
            match r.try_recv()
            {
                Ok(Ok(img)) => {
                    
                    let rx=start_thread_copy_to_clipboard(&img);

                    let em = EditImage::new(img, ctx);
                    frame.set_fullscreen(false);
                    frame.set_visible(true);
                    self.state = EnumGuiState::EditImage(em);
                }
                Err(TryRecvError::Empty) => {show_loading(ctx);},
                Err(TryRecvError::Disconnected) | Ok(Err(_)) => {self.alert.replace("Unable to load the image. please retry"); self.switch_to_main_menu(frame);}
            }
        }else if let EnumGuiState::LoadingEditImage(None) = &mut self.state
        {
            let rx = self.screens_manager.start_thread_fullscreen_screenshot();
            self.state = EnumGuiState::LoadingEditImage(Some(rx));
        }else {unreachable!();}
    }



    fn show_edit_image(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame)
    {
        if let EnumGuiState::EditImage(em) = &mut self.state
        {
            match em.update(ctx, frame, true)
            {
                // todo: manage different formats
                EditImageEvent::Saved {image, format} => 
                {
                    self.save_request = Some((image, format.clone()));

                    self.manage_save_request();
                },
                EditImageEvent::Aborted => { self.switch_to_main_menu(frame)},
                EditImageEvent::Nil => ()
            }
               
        }else {unreachable!();}
    }

    fn manage_save_request(&mut self)
    {
        match (self.save_settings.get_default_dir(), self.save_settings.get_default_name())
        {
            (Some(dp), Some(dn)) => 
            {
                let pb = PathBuf::from(dp);
                let fr = self.save_request.take().unwrap();
                let ext: &str = fr.1.into();
                self.state = EnumGuiState::Saving(image_coding::start_thread_save_image(pb, dn,String::from(ext), fr.0 ));
            }

            (None, Some(dn)) =>
            {
                let dir_opt = file_dialog::show_directory_dialog("");
                if let Some(dir) = dir_opt
                {
                    let fr = self.save_request.take().unwrap();
                    let ext: &str = fr.1.into();
                    self.state = EnumGuiState::Saving(image_coding::start_thread_save_image(dir, dn,String::from(ext), fr.0 ));
                }
            },

            (Some(dp), None) =>
            {
                let fr = self.save_request.take().unwrap();
                let dir_opt = file_dialog::show_save_dialog(&fr.1, Some(&dp.to_string()));
                if let Some(dir) = dir_opt
                {
                    let ext: &str = fr.1.into();
                    let file_name = String::from(dir.file_name().unwrap().to_str().unwrap());
                    self.state = EnumGuiState::Saving(image_coding::start_thread_save_image(dir, file_name,String::from(ext), fr.0 ));
                }
            },

            (None, None) =>
            {
                let fr = self.save_request.take().unwrap();
                let dir_opt = file_dialog::show_save_dialog(&fr.1, None);
                if let Some(dir) = dir_opt
                {
                    let ext: &str = fr.1.into();
                    let file_name = String::from(dir.file_name().unwrap().to_str().unwrap());
                    self.state = EnumGuiState::Saving(image_coding::start_thread_save_image(dir, file_name,String::from(ext), fr.0 ));
                }
            }
        }
    }




    //----------------------SAVING --------------------------------------------------
    fn show_saving(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame)
    {
        if let EnumGuiState::Saving(rx) = &mut self.state
        {
            match rx.try_recv()
            {
                Ok(Ok(_)) =>
                {
                    self.alert.replace("Image saved!");
                    self.switch_to_main_menu(frame);
                },
                Err (TryRecvError::Empty) => show_loading(ctx),
                Err(TryRecvError::Disconnected) | Ok(Err(_)) => {self.alert.replace("Error: image not saved"); self.switch_to_main_menu(frame);}
            }
        }else {unreachable!();}
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
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) 
    {
        if crate::DEBUG {print!("gui refresh. ");}
        
        error_alert::show_error_alert(ctx, &mut self.alert);

        if crate::DEBUG {println!("state = {:?}", self.state);}

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
    }
}
