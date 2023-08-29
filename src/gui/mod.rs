mod main_window;
mod rect_selection;

use eframe::egui;
use std::cell::RefCell;
use main_window::MainWindow;
use rect_selection::RectSelection;
use std::rc::Rc;
use std::cell::RefMut;

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

struct GlobalGuiState
{
    state: Rc<RefCell<EnumGuiState>>
}

impl Clone for GlobalGuiState
{
    fn clone(&self) -> Self
    {
        Self{state: self.state.clone()}
    }
}



impl GlobalGuiState
{
    pub fn new() -> Self
    {
        let ret = Self{state: Rc::new(RefCell::new(EnumGuiState::None))};
        let mw = MainWindow::new(ret.state.clone());
        ret.state.replace(EnumGuiState::ShowingMainWindow(Rc::new(RefCell::new(mw))));
        ret
    }

    /*
    pub fn switch_to_main_window(self: Rc<Self>)
    {
        let mw = MainWindow::new(self.clone());
        self.state.replace(EnumGuiState::ShowingMainWindow(Rc::new(RefCell::new(mw))));
    }

    pub fn switch_to_rect_selection(self: Rc<Self>)
    {
        let rs = RectSelection::new(self.clone());
        self.state.replace(EnumGuiState::ShowingRectSelection(Rc::new(RefCell::new(rs))));
    }
    */
}


pub fn launch_main_window() {  
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
        let temp = self.state.borrow().clone();

        match temp
        {
            EnumGuiState::ShowingMainWindow(mw) => mw.borrow_mut().update(ctx, frame),
            EnumGuiState::ShowingRectSelection(rs) => rs.borrow_mut().update(ctx, frame),
            EnumGuiState::None => ()
        }
    }
}