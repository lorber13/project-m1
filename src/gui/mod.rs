mod main_window;
mod rect_selection;

use eframe::egui;
use std::cell::Cell;
use main_window::MainWindow;
use rect_selection::RectSelection;
use std::rc::Rc;

enum EnumGuiState
{
    ShowingMainWindow(Rc<Cell<MainWindow>>),
    ShowingRectSelection(Rc<Cell<RectSelection>>),
    None
}

struct GlobalGuiState
{
    state: Cell<EnumGuiState>
}

impl GlobalGuiState
{
    pub fn new() -> Self
    {
        let ret = Self{state: Cell::new(EnumGuiState::None)};
        let rc = Rc::new(ret);
        Self::switch_to_main_window(rc);
        ret
    }

    pub fn switch_to_main_window(self: Rc<Self>)
    {
        let mw = MainWindow::new(self.clone());
        self.state.replace(EnumGuiState::ShowingMainWindow(Rc::new(Cell::new(mw))));
    }

    pub fn switch_to_rect_selection(self: Rc<Self>)
    {
        let rs = RectSelection::new(self.clone());
        self.state.replace(EnumGuiState::ShowingRectSelection(Rc::new(Cell::new(rs))));
    }
}


fn launch_main_window() {  
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
        let temp = self.state.get_mut();
        match temp
        {
            EnumGuiState::ShowingMainWindow(mw) => mw.borrow_mut().update(ctx, frame),
            EnumGuiState::ShowingRectSelection(rs) => rs.borrow_mut().update(ctx, frame),
            EnumGuiState::None => ()
        }
    }
}