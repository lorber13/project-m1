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

use eframe::egui;
use eframe::epaint::Rect;
use std::cell::Cell;
use std::cell::RefCell;
use std::io::Write;
use std::io::stderr;
use main_window::MainWindow;
use rect_selection::RectSelection;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::*;

use crate::itc::ScreenshotDim;

pub enum EnumGuiState
{
    ShowingMainWindow(Rc<RefCell<MainWindow>>),
    ShowingRectSelection(Rc<RefCell<RectSelection>>),
    None
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

pub struct GlobalGuiState
{
    state: Rc<RefCell<EnumGuiState>>,
    head_thread_tx: Arc<Sender<super::itc::SignalToHeadThread>>,
    show_alert: Rc<Cell<Option<&'static str>>>
}

impl Clone for GlobalGuiState
{
    fn clone(&self) -> Self
    {
        Self{state: self.state.clone(), head_thread_tx: self.head_thread_tx.clone(), show_alert: self.show_alert.clone()}
    }
}



impl GlobalGuiState
{
    fn new(head_thread_tx: Arc<Sender<super::itc::SignalToHeadThread>>) -> Rc<Self>
    {
        let ret = Rc::new(Self{state: Rc::new(RefCell::new(EnumGuiState::None)), head_thread_tx, show_alert: Rc::new(Cell::new(None))});
        ret.switch_to_main_window();
        ret
    }

    fn switch_to_main_window(self: &Rc<Self>)
    {
        let rs = MainWindow::new(self.clone());
        self.state.replace(EnumGuiState::ShowingMainWindow(Rc::new(RefCell::new(rs))));
    }

    fn switch_to_rect_selection(self: &Rc<Self>)
    {
        let rs = RectSelection::new(self.clone());
        self.state.replace(EnumGuiState::ShowingRectSelection(Rc::new(RefCell::new(rs))));
    }

    fn switch_to_none(self: &Rc<Self>)
    {
        self.state.replace(EnumGuiState::None);
    }

    fn send_acquire_signal(self: &Rc<Self>, sd: ScreenshotDim)
    {
        match self.head_thread_tx.send(crate::itc::SignalToHeadThread::AcquirePressed(sd))
        {
            Ok(_) =>(),
            Err(e) =>
            {
                self.show_alert.set(Some("Impossible to acquire.\nService not available.\nPlease restart the program."));
                writeln!(stderr(), "{}", e);  
            }
        }
    }

    fn send_rect_selected(self: &Rc<Self>, rect: Rect)
    {
        match self.head_thread_tx.send(crate::itc::SignalToHeadThread::RectSelected(rect))
        {
            Ok(_) =>(),
            Err(e) =>
            {
                self.show_alert.set(Some("Impossible to send coordinates of rect.\nService not available.\nPlease restart the program."));
                writeln!(stderr(), "{}", e);  
            }
        }
    }

    
}

struct Gui //wrapper e interfaccia verso l'esterno
{
    ggstate: Rc<GlobalGuiState>
}

impl Gui
{
    fn new(head_thread_tx: Arc<Sender<super::itc::SignalToHeadThread>>) -> Self
    {
        Self{ggstate: GlobalGuiState::new(head_thread_tx)}
    }

    pub fn switch_to_main_window(&self)
    {
        self.ggstate.switch_to_main_window();
    }

    pub fn switch_to_rect_selection(&self)
    {
        self.ggstate.switch_to_rect_selection();
    }

    pub fn show_nothing(&self)
    {
        self.ggstate.switch_to_none();
    }
}


pub fn launch_gui(head_thread_tx: Arc<Sender<super::itc::SignalToHeadThread>>) {  
    let options = eframe::NativeOptions::default();
    
    eframe::run_native(
        "Simple screenshot App", 
        options,  
        Box::new(|_cc| { return Box::new(Gui::new(head_thread_tx)); })
    ).unwrap();
}

impl eframe::App for Gui
{
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) 
    {
        let temp = self.ggstate.state.borrow().clone();

        match temp
        {
            EnumGuiState::ShowingMainWindow(mw) => mw.borrow_mut().update(ctx, frame),
            EnumGuiState::ShowingRectSelection(rs) => rs.borrow_mut().update(ctx, frame),
            EnumGuiState::None => ()
        }

        error_alert::show_error_alert(ctx, self.ggstate.show_alert.clone());

    }
}