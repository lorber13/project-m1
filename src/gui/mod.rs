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
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use eframe::egui::CentralPanel;
use eframe::egui::CursorIcon::Cell;
use image::RgbaImage;
use crate::gui::loading::show_loading;
use crate::itc::ScreenshotDim;
use edit_image::EditImage;

use crate::screenshot::fullscreen_screenshot;

pub enum EnumGuiState
{
    MainWindow(MainWindow),
    RectSelection(Option<RectSelection>, Option<Receiver<Result<RgbaImage, &'static str>>>),
    FileDialog(Option<Receiver<PathBuf>>),
    EditImage(EditImage)

}

impl std::fmt::Debug for EnumGuiState
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error>
    {
        match self
        {
            EnumGuiState::MainWindow(_) => write!(f, "EnumGuiState::MainWindow"),
            EnumGuiState::RectSelection(_, _) => write!(f, "EnumGuiState::RectSelection"),
            EnumGuiState::FileDialog(_) => { todo!() }
            EnumGuiState::EditImage(_) => write!(f, "EnumGuiState::EditImage")
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
    alert: Option<&'static str>,
    current_image: Option<RgbaImage>
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
            current_image: None
        }
    }

    pub fn switch_to_main_window(&mut self)
    {
        self.state = EnumGuiState::MainWindow(MainWindow::new());
    }

    pub fn switch_to_rect_selection(&mut self, frame: &mut eframe::Frame)
    {
        let (tx, rx) = channel();
        frame.set_visible(false);
        thread::spawn(move||{
            thread::sleep(Duration::from_secs(5));
            tx.send(fullscreen_screenshot())
        });
        self.state = EnumGuiState::RectSelection(
            None,
            Some(rx)
        );
    }

    //pub fn switch_to_none(&mut self)
    //{
    //    let mut cv = Arc::new((Condvar::new(), Mutex::new(false)));
    //    let mut guard = self.state.lock().unwrap();
    //    *guard = EnumGuiState::None(cv.clone());
    //    drop(guard);
    //    cv.0.wait_while(cv.1.lock().unwrap(), |sig| !*sig);
    //}

    pub fn show_file_dialog(&mut self)
    {
        let wait = thread::spawn(||
        {
            file_dialog::show_file_dialog() // -> Option<PathBuf>
        });
        self.state = EnumGuiState::FileDialog(wait);
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
                            self.switch_to_rect_selection(frame);
                        }
                    }
                }
            }
    }

    fn show_rect_selection(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)
    {
        if let EnumGuiState::RectSelection(opt_rs, opt_r) = &mut self.state
        {
            if let Some(r) = opt_r
            {
                if let Ok(msg) = r.try_recv()
                {
                    frame.set_visible(true);
                    match msg {
                        Ok(img) => {
                            let rs = RectSelection::new(&img);
                            self.state = EnumGuiState::RectSelection(Some(rs), None);
                        }
                        Err(error_message) => {
                            self.alert = Some(error_message)
                        }
                    }
                } else {
                    show_loading(ctx);
                }
            } else if let Some(ref mut rs) = opt_rs
            {
                if let Some(screenshot) = rs.update(ctx, frame) {
                    self.current_image = Some(screenshot);
                };
            } else {
                unreachable!();
            }
        }
    }

    fn show_edit_image(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)
    {

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


        // todo move the code in a dedicated function
        match &mut self.state
        {
            EnumGuiState::MainWindow(_) => {
                self.show_main_window(ctx, frame);
            },
            EnumGuiState::RectSelection(_, _) => {
                    self.show_rect_selection(ctx, frame);
            },
            EnumGuiState::FileDialog(_) => { }, // todo
            EnumGuiState::EditImage(_) =>
                {
                    self.show_edit_image(ctx, frame);
                }
        }
    }
}