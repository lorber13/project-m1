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
use std::fmt::Formatter;
use std::path::PathBuf;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use image::RgbaImage;

use crate::screenshot::fullscreen_screenshot;

pub enum EnumGuiState
{
    ShowingMainWindow(MainWindow),
    ShowingRectSelection(Option<RectSelection>, Option<JoinHandle<Result<RgbaImage, &'static str>>>),
    ShowingFileDialog(JoinHandle<Option<PathBuf>>)
}

impl std::fmt::Debug for EnumGuiState
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error>
    {
        match self
        {
            EnumGuiState::ShowingMainWindow(_) => write!(f, "EnumGuiState::ShowingMainWindow"),
            EnumGuiState::ShowingRectSelection(_, _) => write!(f, "EnumGuiState::ShowingRectSelection"),
            EnumGuiState::ShowingFileDialog(_) => { todo!() }
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
            state: EnumGuiState::ShowingMainWindow(MainWindow::new()),
            alert: None,
            current_image: None
        }
    }

    pub fn switch_to_main_window(&mut self)
    {
        self.state = EnumGuiState::ShowingMainWindow(MainWindow::new());
    }

    pub fn switch_to_rect_selection(&mut self)
    {
        let wait = thread::spawn(move||{
            thread::sleep(Duration::from_secs(5));
            fullscreen_screenshot()
        });
        self.state = EnumGuiState::ShowingRectSelection(
            None,
            Some(wait)
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

    pub fn show_error_alert(&mut self, s: &'static str) // todo: it is called by head_thread only. Do we still need it?
    {
        self.alert.replace(s);
    }

    pub fn show_file_dialog(&mut self)
    {
        let wait = thread::spawn(||
        {
            file_dialog::show_file_dialog() // -> Option<PathBuf>
        });
        self.state = EnumGuiState::ShowingFileDialog(wait);
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
            EnumGuiState::ShowingMainWindow(ref mut mw) => {
                mw.update(ctx, frame); // todo: update() returns an option
            },
            EnumGuiState::ShowingRectSelection(opt_rs, opt_r) =>
                {
                    if let Some(r) = opt_r.take() // todo: it is a workaround to take ownership of the value inside the option
                    {
                        if r.is_finished()
                        {
                            if let Ok(ret) = r.join() {
                                if let Ok(img) = ret {
                                    let rs = RectSelection::new(&img);
                                    self.state = EnumGuiState::ShowingRectSelection(Some(rs), None);
                                }
                            }
                        } else
                        {
                            todo!() // show_loading
                        }
                    } else if let Some(ref mut rs) = opt_rs
                    {
                        if let Some(screenshot) = rs.update(ctx, frame) {
                            self.current_image = Some(screenshot);
                        };
                    } else {
                        unreachable!();
                    }
                },
            EnumGuiState::ShowingFileDialog(_) => { } // todo
        }
    }
}