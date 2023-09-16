/*
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

use eframe::egui;
use eframe::egui::Vec2;
use eframe::epaint::mutex::Mutex;
use main_window::MainWindow;
use rect_selection::RectSelection;
use std::collections::VecDeque;
use std::fmt::Formatter;
use std::path::PathBuf;
use std::sync::Condvar;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::thread;
use eframe::egui::Rect;
use image::{RgbaImage, ImageError};
use crate::{DEBUG, image_coding, screenshot};
use crate::gui::loading::show_loading;
use crate::image_coding::{copy_to_clipboard, ImageFormat};
use crate::itc::ScreenshotDim;
use edit_image::EditImage;

use self::edit_image::EditImageEvent;

pub enum EnumGuiState
{
    MainWindow(MainWindow),
    LoadingRectSelection(Option<Receiver<Result<RgbaImage, &'static str>>>),
    RectSelection(RectSelection),
    LoadingEditImage(Option<Receiver<Result<RgbaImage, &'static str>>>),
    EditImage(EditImage, Option<Receiver<Option<PathBuf>>>),
    Saving(Receiver<Result<(), ImageError>>)
}

impl std::fmt::Debug for EnumGuiState
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error>
    {
        match self
        {
            EnumGuiState::MainWindow(_) => write!(f, "EnumGuiState::MainWindow"),
            EnumGuiState::LoadingRectSelection(..) => write!(f, "EnumGuiState::LoadingRectSelection"),
            EnumGuiState::RectSelection(..) => write!(f, "EnumGuiState::RectSelection"),
            EnumGuiState::EditImage(..) => write!(f, "EnumGuiState::EditImage"),
            EnumGuiState::LoadingEditImage(_) => write!(f, "EnumGuiState::LoadingEdiImage"),
            EnumGuiState::Saving(_) => write!(f, "EnumGuiState::Start")
        }
    }
}

/*
impl Clone for EnumGuiState
{
    fn clone(&self) -> Self 
    {
        match self
        {
            Self::ShowingMainWindow(rc) => Self::ShowingMainWindow(rc.clone()),
            Self::ShowingRectSelection(rc) => Self::ShowingRectSelection(rc.clone()),
            Self::None(cv) => Self::None(cv.clone())
        }
    }
}
*/

pub struct GlobalGuiState
{
    state: EnumGuiState,
    alert: Option<&'static str>,
    save_request: Option<(RgbaImage, ImageFormat)>,
    screens_manger: screenshot::ScreensManager
}

/*
impl Clone for GlobalGuiState
{
    fn clone(&self) -> Self
    {
        Self{state: self.state.clone(), show_alert: self.show_alert.clone(), 
                show_file_dialog: self.show_file_dialog.clone(),
                head_thread_tx: self.head_thread_tx.clone()}
    }
}
*/



impl GlobalGuiState
{
    fn new() -> Self
    {
        GlobalGuiState {
            state: EnumGuiState::MainWindow(MainWindow::new()),
            alert: None,
            save_request: None,
            screens_manger: screenshot::ScreensManager::new(150)
        }
    }





    /*----------------MAIN WINDOW------------------------------------------ */

    fn switch_to_main_window(&mut self,  _frame: &mut eframe::Frame)
    {
        _frame.set_decorations(true);
        _frame.set_fullscreen(false);
        _frame.set_maximized(false);
        _frame.set_window_size(egui::Vec2::new(500.0, 300.0));
        _frame.set_visible(true);
        self.state = EnumGuiState::MainWindow(MainWindow::new());
    }

    fn show_main_window(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let EnumGuiState::MainWindow(ref mut mw) = self.state
            {
                if let Some((request, screen_id)) = mw.update(&mut self.screens_manger, ctx, frame) {
                    match request {
                        ScreenshotDim::Fullscreen => {
                            self.switch_to_edit_image(None, ctx, frame);
                        }
                        ScreenshotDim::Rectangle => {
                            self.switch_to_rect_selection(ctx, frame);
                        }
                    }
                }
            }
    }






    /*--------------RECT SELECTION---------------------------------------- */

    fn switch_to_rect_selection(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)
    {
        frame.set_visible(false);
        ctx.request_repaint();
        
        self.state = EnumGuiState::LoadingRectSelection(None);
    }

    fn load_rect_selection(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)
    {
        match &mut self.state
        {
            EnumGuiState::LoadingRectSelection(None) => //il thread non è ancora stato spawnato
            {
                let (tx, rx) = channel();
                let sc = self.screens_manger.clone();
                    thread::spawn(move||{

                        tx.send(sc.fullscreen_screenshot());
                    });
                    self.state = EnumGuiState::LoadingRectSelection(Some(rx));
                
            },

            EnumGuiState::LoadingRectSelection(Some(r)) => //in attesa che il thread invii l'immmagine
            {
                //se sono in stato di attesa, controllo se il thread worker ha inviato sul canale
                match r.try_recv()
                {
                    //se un messaggio è stato ricevuto, interrompo lo stato di attesa e visualizzo la prossima schermata
                    Ok(msg) =>
                    {
                        frame.set_visible(true);
                        frame.set_fullscreen(true);
                        //ctx.request_repaint();
                        match msg {
                            Ok(img) => {
                                let rs = RectSelection::new(img, ctx);
                                self.state = EnumGuiState::RectSelection(rs);
                                //ctx.request_repaint(); //necessario per rendere dinuovo il frame visibile (dopo averlo reso invisibile)
                            }
                            Err(error_message) => {
                                self.alert = Some(error_message)
                            }
                        }
                    },

                    Err(TryRecvError::Disconnected) => {
                        frame.set_visible(true);
                        self.alert.replace("An error occoured when trying to start the service. Please retry.");
                        self.switch_to_main_window(frame);
                    },
                    Err(TryRecvError::Empty) => show_loading(ctx)
                }
            },

            _ => unreachable!()
        }
        
        
    }


    fn show_rect_selection(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)
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
    fn switch_to_edit_image(&mut self, opt_rect_img: Option<(Rect, RgbaImage)>, ctx: &egui::Context, frame: &mut eframe::Frame)
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


    fn load_edit_image(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)
    {
        if let EnumGuiState::LoadingEditImage(Some(r)) = &mut self.state //attesa dell'immagine da caricare
        {
            match r.try_recv()
            {
                Ok(Ok(img)) => {
                    copy_to_clipboard(&img);
                    let em = EditImage::new(img, ctx);
                    frame.set_fullscreen(false);
                    frame.set_visible(true);
                    self.state = EnumGuiState::EditImage(em, None);
                }
                Err(TryRecvError::Empty) => {show_loading(ctx);},
                Err(TryRecvError::Disconnected) | Ok(Err(_)) => {self.alert.replace("Unable to load the image. please retry"); self.switch_to_main_window(frame);}
            }
        }else if let EnumGuiState::LoadingEditImage(None) = &mut self.state
        {
            let (tx, rx) = channel();
            let sc = self.screens_manger.clone();
            thread::spawn(move||
                {
                    tx.send(sc.fullscreen_screenshot());
                });
            self.state = EnumGuiState::LoadingEditImage(Some(rx));
        }else {unreachable!();}
    }



    fn show_edit_image(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)
    {
        match &mut self.state
        {
            EnumGuiState::EditImage(ref mut em, None) => //non c'è attesa su nessun canale: aggiorno normalmente la finestra
            {
                match em.update(ctx, frame, true)
                {
                    // todo: manage different formats
                    EditImageEvent::Saved {image, format} => 
                    {
                        self.save_request = Some((image, format.clone()));
                        self.start_file_dialog(format)
                    },
                    EditImageEvent::Aborted => { self.switch_to_main_window(frame)},
                    EditImageEvent::Nil => ()
                }
            }
            EnumGuiState::EditImage(em, Some(r)) => //il file dialog è aperto
            {
                self.wait_file_dialog(ctx, frame);
            },
    
            _ => {unreachable!();}
           
        }
    }

    fn wait_file_dialog(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)
    {
        if let EnumGuiState::EditImage(em, opt_rx) =  &mut self.state
        {
           if let Some(rx) = opt_rx
           {
                match rx.try_recv() //controllo se il thread ha già inviato attraverso il canale
                {
                    Ok(pb_opt) => {   //in caso sia già stato ricevuto un messaggio, esso può essere a sua volta None (se l'utente ha deciso di annullare il salvataggio)
                        *opt_rx = None;
                        match pb_opt
                        {
                            Some(pb) => 
                            {
                                let rq = self.save_request.take().expect("self.save_request must not be empty");
                                let file_output = pb.with_extension::<&'static str>(rq.1.into());
                                let rx = image_coding::start_thread_save_image(file_output, rq.0);
                                self.state = EnumGuiState::Saving(rx);
                            },
                            None => { *opt_rx = None; }   //se l'operazione è stata annullata, si torna a image editing
                        }
                    },

                    Err(TryRecvError::Empty) => {   //in caso non sia ancora stato ricevuto il messaggio, continuo a mostrare la finestra precedente disabilitata
                        em.update(ctx, frame, false);
                    },

                    Err(TryRecvError::Disconnected) => {    //in caso il thread sia fallito, segnalo errore e torno a mostrare EditImage
                        *opt_rx = None;
                        self.alert.replace("Error in file dialog. Please retry.");
                    }
                }
           }
        }
    }

    pub fn start_file_dialog(&mut self, format: ImageFormat) 
    {
        let (tx, rx) = channel::<Option<PathBuf>>();
        std::thread::spawn(move ||
            {
                tx.send(file_dialog::show_file_dialog(format))
            });

        if let EnumGuiState::EditImage(_, ref mut r_opt ) = self.state
        {
            *r_opt = Some(rx);
        }
        
    }




    //----------------------SAVING --------------------------------------------------
    fn show_saving(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)
    {
        if let EnumGuiState::Saving(rx) = &mut self.state
        {
            match rx.try_recv()
            {
                Ok(Ok(res)) =>
                {
                    self.alert.replace("Image saved!");
                    self.switch_to_main_window(frame);
                },
                Err (TryRecvError::Empty) => show_loading(ctx),
                Err(TryRecvError::Disconnected) | Ok(Err(_)) => {self.alert.replace("Error: image not saved"); self.switch_to_main_window(frame);}
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
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) 
    {
        if crate::DEBUG {print!("gui refresh. ");}
        
        error_alert::show_error_alert(ctx, &mut self.alert);

        if crate::DEBUG {println!("state = {:?}", self.state);}

        match &mut self.state
        {
            EnumGuiState::MainWindow(_) => {
                self.show_main_window(ctx, frame);
            },
            EnumGuiState::LoadingRectSelection(_) =>
            {
                self.load_rect_selection(ctx, frame);
            }
            EnumGuiState::RectSelection(..) => {
                    self.show_rect_selection(ctx, frame);
            }, 
            EnumGuiState::LoadingEditImage(_) =>
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