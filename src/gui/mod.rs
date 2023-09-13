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
use main_window::MainWindow;
use rect_selection::RectSelection;
use std::fmt::Formatter;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::thread;
use eframe::egui::Rect;
use image::RgbaImage;
use crate::gui::loading::show_loading;
use crate::itc::ScreenshotDim;
use edit_image::EditImage;

use crate::screenshot::fullscreen_screenshot;

use self::edit_image::EditImageEvent;

pub enum EnumGuiState
{
    MainWindow(MainWindow),
    RectSelection(Option<RectSelection>, Option<Receiver<Result<RgbaImage, &'static str>>>),
    EditImage(Option<EditImage>, Option<Receiver<RgbaImage>>, Option<Receiver<Option<PathBuf>>>),
}

impl std::fmt::Debug for EnumGuiState
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error>
    {
        match self
        {
            EnumGuiState::MainWindow(_) => write!(f, "EnumGuiState::MainWindow"),
            EnumGuiState::RectSelection(..) => write!(f, "EnumGuiState::RectSelection"),
            EnumGuiState::EditImage(..) => write!(f, "EnumGuiState::EditImage")
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


#[derive(Debug)]
pub struct GlobalGuiState
{
    state: EnumGuiState,
    alert: Option<&'static str>
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
            alert: None
        }
    }

    fn switch_to_main_window(&mut self)
    {
        self.state = EnumGuiState::MainWindow(MainWindow::new());
    }

    fn switch_to_rect_selection(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)
    {
        frame.set_visible(false);
        self.state = EnumGuiState::RectSelection(
            None,
            None
        );
    }

    //uso il rettangolo per ritagliare l'immagine precedentemente salvata
    //un thread worker esegue il task, mentre la gui mostrerà la schermata di caricamento
    fn switch_to_edit_image(&mut self, rect: Rect, img: RgbaImage)
    {
        let (tx, rx) = channel();
        thread::spawn(move||
            {
                let crop_img = image::imageops::crop_imm::<RgbaImage>(&img,
                                                                            rect.left() as u32,
                                                                            rect.top() as u32,
                                                                            rect.width() as u32,
                                                                            rect.height() as u32).to_image();


                let _ = tx.send(crop_img);
            });
        // passo nello stadio di attesa dell'immagine ritagliata (non sono ancora dentro editImage)
        self.state = EnumGuiState::EditImage(None, Some(rx), None);
    }

    //pub fn switch_to_none(&mut self)
    //{
    //    let mut cv = Arc::new((Condvar::new(), Mutex::new(false)));
    //    let mut guard = self.state.lock().unwrap();
    //    *guard = EnumGuiState::None(cv.clone());
    //    drop(guard);
    //    cv.0.wait_while(cv.1.lock().unwrap(), |sig| !*sig);
    //}

    pub fn start_file_dialog(&mut self) 
    {
        let (tx, rx) = channel::<Option<PathBuf>>();
        std::thread::spawn(move ||
            {
                tx.send(file_dialog::show_file_dialog())
            });

        if let EnumGuiState::EditImage(_, _, ref mut r_opt ) = self.state
        {
            *r_opt = Some(rx);
        }
        
    }

    fn show_main_window(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let EnumGuiState::MainWindow(ref mut mw) = self.state
            {
                if let Some(request) = mw.update(ctx, frame) {
                    match request {
                        ScreenshotDim::Fullscreen => {
                            todo!()
                        }
                        ScreenshotDim::Rectangle => {
                            self.switch_to_rect_selection(ctx, frame);
                        }
                    }
                }
            }
    }

    fn show_rect_selection(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)
    {
        if let EnumGuiState::RectSelection(None, None) = self.state
        {
            let (tx, rx) = channel();
            thread::spawn(move||{
                tx.send(fullscreen_screenshot());
            });
            self.state = EnumGuiState::RectSelection(None, Some(rx));
        }else if let EnumGuiState::RectSelection(opt_rs, opt_r) = &mut self.state
        {
            //controllo se sono in stato di loading, lo stato di loading è segnalato da Some(Receiver) nel secondo campo della tupla
            if let Some(r) = opt_r
            {
                //se sono in stato di attesa, controllo se il thread worker ha inviato sul canale
                match r.try_recv()
                {
                    //se un messaggio è stato ricevuto, interrompo lo stato di attesa e visualizzo la prossima schermata
                    Ok(msg) =>
                    {
                        frame.set_visible(true);
                        match msg {
                            Ok(img) => {
                                let rs = RectSelection::new(img, ctx);
                                self.state = EnumGuiState::RectSelection(Some(rs), None);
                            }
                            Err(error_message) => {
                                self.alert = Some(error_message)
                            }
                        }
                    },

                    Err(TryRecvError::Disconnected) => {
                        self.alert.replace("An error occoured when trying to start the service. Please retry.");
                        self.switch_to_main_window();
                    },
                    Err(TryRecvError::Empty) => show_loading(ctx)
                }

            } //se non sono in stato di attesa, mostro la schermata di rect selection
            else if let Some(ref mut rs) = opt_rs
            {
                if let Some((rect, rgba)) = rs.update(ctx, frame) {
                    self.switch_to_edit_image(rect, rgba);
                };
            } else {
                unreachable!();
            }
        }
    }

    fn show_edit_image(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)
    {
        match &mut self.state
        {
            EnumGuiState::EditImage(Some(ref mut em),None , None) => //non c'è attesa su nessun canale: aggiorno normalmente la finestra
            {
                match em.update(ctx, frame, true)
                {
                    // todo: manage different formats
                    EditImageEvent::Saved {..} => self.start_file_dialog(),
                    EditImageEvent::Aborted => { self.switch_to_main_window()},
                    EditImageEvent::Nil => ()
                }
            }
            EnumGuiState::EditImage(Some(em), None , Some(r)) => //il file dialog è aperto
            {
                self.wait_file_dialog(ctx, frame);
            },
            EnumGuiState::EditImage(None, Some(r), None) => //attesa dell'immagine da caricare
            {
                if crate::DEBUG {println!("DEBUG: EditImage(None, Some(Receiver<RgbaImage>, None)")}
                match r.try_recv()
                {
                    Ok(img) => {
                        let em = EditImage::new(img, ctx);
                        frame.set_fullscreen(false);
                        self.state = EnumGuiState::EditImage(Some(em), None, None);
                    }
                    Err(TryRecvError::Empty) => {show_loading(ctx);},
                    Err(TryRecvError::Disconnected) => {self.alert.replace("Unable to load the image. please retry"); self.switch_to_main_window();}
                }
            }
            _ => {unreachable!();}
           
        }
    }

    fn wait_file_dialog(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)
    {
        if let EnumGuiState::EditImage(Some(em),_ , ref mut opt_rx) =  &mut self.state
        {
           if let Some(rx) = opt_rx
           {
                match rx.try_recv() //controllo se il thread ha già inviato attraverso il canale
                {
                    Ok(pb_opt) => {   //in caso sia già stato ricevuto un messaggio, esso può essere a sua volta None (se l'utente ha deciso di annullare il salvataggio)
                        *opt_rx = None;
                        match pb_opt
                        {
                            Some(pb) => show_loading(ctx),  //TODO: spawnare il thread che effettua il salvataggio
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
            EnumGuiState::RectSelection(..) => {
                    self.show_rect_selection(ctx, frame);
            }, 
            EnumGuiState::EditImage(..) =>
                {
                    self.show_edit_image(ctx, frame);
                }
        }
    }
}