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

use eframe::egui;
use main_window::MainWindow;
use rect_selection::RectSelection;
use std::sync::{Arc, Mutex, Condvar};
use std::sync::mpsc::*;
use std::fmt::Formatter;
use egui::Vec2;
use std::rc::Rc;
use eframe::egui::Pos2;
use egui_extras::RetainedImage;
use image::RgbaImage;

use crate::DEBUG;
use crate::screenshot::fullscreen_screenshot;

pub enum EnumGuiState
{
    ShowingMainWindow(Option<MainWindow>),
    ShowingRectSelection((Option<RectSelection>, Option<Receiver<RgbaImage>>)),
    ShowingFileDialog(Receiver<std::path::PathBuf>),
    None(Arc<(Condvar, Mutex<bool>)>)
}

impl std::fmt::Debug for EnumGuiState
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error>
    {
        match self
        {
            EnumGuiState::ShowingMainWindow(_) => write!(f, "EnumGuiState::ShowingMainWindow"),
            EnumGuiState::ShowingRectSelection(_) => write!(f, "EnumGuiState::ShowingRectSelection"),
            EnumGuiState::None(_) => write!(f, "EnumGuiState::None")
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
    show_alert: Option<&'static str>
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
    fn new(head_thread_tx: Arc<Mutex<Sender<super::itc::SignalToHeadThread>>>) -> Self
    {
        let mut cv = Arc::new((Condvar::new(), Mutex::new(true)));
        let ret = Rc::new(Self{state: EnumGuiState::None(cv),
                                                    show_alert: None});
        ret.clone().switch_to_main_window();
        ret
    }

    pub fn switch_to_main_window(&mut self)
    {
        let rs = MainWindow::new(self.clone(), self.head_thread_tx.clone());
        let mut guard = self.state.lock().unwrap();
        *guard = EnumGuiState::ShowingMainWindow(rs);
    }

    pub fn switch_to_rect_selection(&mut self)
    {
        let (rx, tx) = std::sync::channel();
        std::thread::spawn(move||{let img = fullscreen_screenshot(); tx.send(img);})
        self.state = EnumGuiState::ShowingRectSelection((None, Some(rx)));
    }

    pub fn switch_to_none(&mut self)
    {
        let mut cv = Arc::new((Condvar::new(), Mutex::new(false)));
        let mut guard = self.state.lock().unwrap();
        *guard = EnumGuiState::None(cv.clone());
        drop(guard);
        cv.0.wait_while(cv.1.lock().unwrap(), |sig| !*sig);
    }

    pub fn show_error_alert(&mut self s: &'static str)
    {
        self.clone().show_alert.lock().unwrap().replace(s);
    }

    pub fn show_file_dialog(&mut self)
    {
        let tx = self.head_thread_tx.clone();
        std::thread::spawn(move ||
        {
            let ret = file_dialog::show_file_dialog();
            tx.lock().unwrap().send(crate::itc::SignalToHeadThread::PathSelected(ret));
        });
    }
    
}



pub fn launch_gui(ggstate: Arc<GlobalGuiState>)
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
        
        error_alert::show_error_alert(ctx, self.ggstate.show_alert.clone());

        if crate::DEBUG {println!("state = {:?}", self.state);}


        match self.state
        {
            EnumGuiState::ShowingMainWindow(mw) => {mw.update(ctx, frame); },
            EnumGuiState::ShowingRectSelection((opt_rs, opt_r)) =>
                {
                    if let Some(r) = opt_r
                    {
                        if let Ok(img) = r.try_recv()
                        {
                            let mut rs = RectSelection::new(img);
                            self.state = EnumGuiState::ShowingRectSelection((Some(rs), None));

                        }else
                        {
                                self.show_loading();
                        }
                    }else if let Some(rs) = opt_rs
                    {
                        rs.update(ctx, frame);
                    }
                },
            EnumGuiState::None(cv) => 
            {
                frame.set_window_size(Vec2::ZERO); frame.set_decorations(false);
                let mut guard = cv.1.lock().unwrap();
                *guard = true;
                cv.0.notify_all(); 
            }
        }
    }
}