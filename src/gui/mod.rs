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
use std::sync::{Arc, Mutex};
use std::sync::mpsc::*;
use std::fmt::Formatter;
use egui::Vec2;

use crate::DEBUG;

pub enum EnumGuiState
{
    ShowingMainWindow(Arc<Mutex<MainWindow>>),
    ShowingRectSelection(Arc<Mutex<RectSelection>>),
    None
}

impl std::fmt::Debug for EnumGuiState
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error>
    {
        match self
        {
            EnumGuiState::ShowingMainWindow(_) => write!(f, "EnumGuiState::ShowingMainWindow"),
            EnumGuiState::ShowingRectSelection(_) => write!(f, "EnumGuiState::ShowingRectSelection"),
            EnumGuiState::None => write!(f, "EnumGuiState::None")
        }
    }
}

impl Clone for EnumGuiState
{
    fn clone(&self) -> Self 
    {
        match self
        {
            Self::ShowingMainWindow(rc) => Self::ShowingMainWindow(rc.clone()),
            Self::ShowingRectSelection(rc) => Self::ShowingRectSelection(rc.clone()),
            Self::None => Self::None
        }
    }
}

#[derive(Debug)]
pub struct GlobalGuiState
{
    state: Arc<Mutex<EnumGuiState>>,
    show_alert: Arc<Mutex<Option<&'static str>>>,
    head_thread_tx: Arc<Mutex<Sender<crate::itc::SignalToHeadThread>>>
}

impl Clone for GlobalGuiState
{
    fn clone(&self) -> Self
    {
        Self{state: self.state.clone(), show_alert: self.show_alert.clone(), 
                head_thread_tx: self.head_thread_tx.clone()}
    }
}



impl GlobalGuiState
{
    fn new(head_thread_tx: Arc<Mutex<Sender<super::itc::SignalToHeadThread>>>) -> Arc<Self>
    {
        let ret = Arc::new(Self{state: Arc::new(Mutex::new(EnumGuiState::None)), 
                                                            head_thread_tx, 
                                                            show_alert: Arc::new(Mutex::new(None))});
        ret.clone().switch_to_main_window();
        ret
    }

    pub fn switch_to_main_window(self: Arc<Self>)
    {
        let rs = MainWindow::new(self.clone(), self.head_thread_tx.clone());
        let mut guard = self.state.lock().unwrap();
        *guard = EnumGuiState::ShowingMainWindow(Arc::new(Mutex::new(rs)));
    }

    pub fn switch_to_rect_selection(self: Arc<Self>)
    {
        if DEBUG {println!("invoking RectSelection::new()");}
        let rs = RectSelection::new(self.clone(), self.head_thread_tx.clone());
        if DEBUG {println!("trying to acquire lock to switch to rect selection: {:?}", self.state);}
        let mut guard = self.state.lock().unwrap();
        *guard = EnumGuiState::ShowingRectSelection(Arc::new(Mutex::new(rs)));
        if DEBUG {println!("lock acquired. State changed in: {:?}", *guard);}
    }

    pub fn switch_to_none(&self)
    {
        let mut guard = self.state.lock().unwrap();
        *guard = EnumGuiState::None;
    }

    
}

struct GuiWrapper //solo wrapper per GlobalGuiState
{
    ggstate: Arc<GlobalGuiState>
}

impl GuiWrapper
{
    fn new(head_thread_tx: Arc<Mutex<Sender<super::itc::SignalToHeadThread>>>) -> Self
    {
        Self{ggstate: GlobalGuiState::new(head_thread_tx)}
    }
}

pub fn new_gui(head_thread_tx: Arc<Mutex<Sender<super::itc::SignalToHeadThread>>>) -> Arc<GlobalGuiState>
{
    GlobalGuiState::new(head_thread_tx)
}


pub fn launch_gui(ggstate: Arc<GlobalGuiState>)
{  
    let options = eframe::NativeOptions::default();
    let wrapper = GuiWrapper{ggstate};
    eframe::run_native(
        "Simple screenshot App", 
        options,  
        Box::new(|_cc| { return Box::new(wrapper); })
    ).unwrap();
}

impl eframe::App for GuiWrapper
{
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) 
    {
        if crate::DEBUG {print!("gui refresh. ");}
        
        error_alert::show_error_alert(ctx, self.ggstate.show_alert.clone());
        let temp = self.ggstate.state.lock().unwrap().clone();

        if crate::DEBUG {println!("state = {:?}", temp);}


        match temp
        {
            EnumGuiState::ShowingMainWindow(mw) => {let mut g = mw.lock().unwrap(); g.update(ctx, frame); },
            EnumGuiState::ShowingRectSelection(rs) => { let mut g = rs.lock().unwrap();g.update(ctx, frame); },
            EnumGuiState::None => {frame.set_window_size(Vec2::ZERO); frame.set_decorations(false); }
        }
    }
}