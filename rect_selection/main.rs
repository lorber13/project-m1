/*
    Codice per permettere all'utente di selezionare un'area (rettangolo) in cui fare lo screenshot.
    DEVE ESSERE ESEGUITO IN UN PROCESSO SEPARATO, che avrà la su tutti gli altri processi in esecuzione nel sistema, 
    grazie all'opzione always_on_top.
    E' necessario un nuovo processo, e non è sufficiente un nuovo thread, per avere garanzia che l'eframe corrente si 
    possa sovrapporre a tutti gli altri in esecuzione e perchè si vuole dare la possibilità all'utente di chiudere
    il processo di selezione in qualsiasi momento, cliccando su un pulsante ABORT, che terminerà appunto il processo (std::process::exit()).

    Il processo può terminare quindi in modi diversi, come descritto dall'enum ExitCode
*/
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

const DEBUG: bool = true; //if true, it prints messages on console
const DISPLAY_HEIGHT: f32 = 1080.0;
const DISPLAY_WIDTH: f32 = 1919.0;
pub enum ExitCode
{
    ABORT = 1,
    SELECTED = 0,
    ERROR = -1
}

impl Into<i32> for ExitCode
{
    fn into(self) -> i32
    {
        match self 
        {
            ExitCode::ABORT => 1,
            ExitCode::ERROR => -1,
            ExitCode::SELECTED => 0
        }
    }
}



use eframe::egui;
use crate::egui::Pos2;
use crate::egui::Color32;
use crate::egui::Rect;
use crate::egui::Stroke;
use crate::egui::Rounding;

//dati due punti nello spazio, le coordinate (x, y) sono riordinate per identificare gli angoli top-sx e bottom-dx del rettangolo
fn rect_from_pos2(p1: &Pos2, p2: &Pos2) -> Rect
{
    let min_x = if p1.x <p2.x { p1.x }else{ p2.x};
    let min_y = if p1.y <p2.y { p1.y }else{ p2.y};
    let left_top = Pos2::new(min_x.round(), min_y.round());
    let max_x = if p1.x >p2.x { p1.x }else{ p2.x};
    let max_y = if p1.y >p2.y { p1.y }else{ p2.y};
    let right_bottom = Pos2::new(max_x.round(), max_y.round());

    Rect { min: left_top, max: right_bottom }
}


fn main() -> Result<(), eframe::Error> {

    let options = eframe::NativeOptions {
        decorated : false,
        transparent: true,
        always_on_top : true,
        //fullscreen: true,     //NO! SI PERDE LA TRASPARENZA
        maximized: true,
        ..Default::default()
    };

    // Stato dell'applicazione: le coordinate dei punti selezionati durante drag & drop
    let mut state : [Option<Pos2>;2] = [None, None];       

    eframe::run_simple_native("Area selection", options, move |ctx, _frame| {

        //finestra fullscreen trasparente
        egui::Window::new("Area selection")
        .movable(false) //per impedire che le operazioni di drag & drop spostino la finestra
        .frame(egui::Frame::none().fill(Color32::TRANSPARENT))
        .show(ctx, |ui|
        {
            ui.set_min_height(DISPLAY_HEIGHT);  //dimesioni hard coded (non rilevate in automatico)
            ui.set_min_width(DISPLAY_WIDTH);

            //BOTTONI: Abort, Confirm, Reset
            ui.horizontal(|ui2| 
            {
                if ui2.button("Abort").clicked()
                {
                    std::process::exit(ExitCode::ABORT.into());
                }
                if ui2.button("Confirm").clicked()
                {
                    match state
                    {
                        [Some(_), Some(_)] => 
                        {
                            //TODO: codice per restituire le coordinate del rettangolo selezionato al processo padre
                            if DEBUG { println!("state = {:?}", state); }
                            std::process::exit(ExitCode::SELECTED.into())
                        },
                        _ => ()
                    }
                    
                }
                
                if ui2.button("Reset").clicked()
                {
                    state = [None, None];
                }
            });
            

            //Contentitore per disegnare le forme e intercettare gli eventi click e drag
            let (space, painter) = ui.allocate_painter(
                egui::Vec2::new(ui.available_width(), ui.available_height()),
                egui::Sense::click_and_drag(),
            );

            //se lo stato contiene le coordinate di due punti, disegno il rettangolo corrispondente
            if let [Some(p1), Some(p2)] = state
            {
                painter.rect(rect_from_pos2(&p1, &p2), Rounding::none(), Color32::from_white_alpha(10), Stroke::NONE);
            }

            if space.clicked() //se è avvenuto solo click e non drag, si resetta lo stato
            {
                state = [None, None];
                if DEBUG { println!("state = {:?}", state); }
            }else if space.drag_started() && !space.drag_released() //se inizio del drag, si memorizzano le coordinate del puntatore
            {
                 match space.hover_pos()
                {
                    None => (),
                    Some(p_not_round) =>
                    {
                        let p = p_not_round.round();
                        match state
                        {
                            [None, None] | [Some(_), None]=> 
                            {
                                state = [Some(p), None];
                                if DEBUG { println!("state = {:?}", state); }
                            },  
                            [None, Some(_)] => 
                            {
                                //ERRORE: per rimediare si resetta lo stato
                                if DEBUG {println!("DEBUG: error: state = [None, Some]")}
                                state = [None, None];
                            },
                            [Some(_), Some(_) ] =>
                            {
                                //si riavvia un nuovo drag dopo averne terminato uno
                                state = [Some(p), None];
                            }
                        }
                    } 
                        
                }
            }else if space.drag_released() //se è terminato il drag, si memorizza la posizione del puntatore
            {
                if let Some(p_not_round) = space.hover_pos()
                {
                    let p2 = p_not_round.round();
                    match state
                    {
                        [Some(p1), None] => 
                        {
                            //se il drag è stato rilasciato, necessariamente lo stato deve contenere già un punto
                            state = [Some(p1), Some(p2)];
                            if DEBUG { println!("state = {:?}", state); }
                        },
                        _ => 
                        {
                            //ERRORE: per rimediare si resetta lo stato
                            if DEBUG {println!("DEBUG: error: state = [None, Some]")}
                            state = [None, None];
                        }
                    }
                }
            }else   //durante il drag & drop (quindi, solo se lo stato contiene già il primo punto), si disegna il rettangolo 
            {     
                match space.hover_pos()
                {
                    Some(p_not_round) =>
                    {
                        let p = p_not_round.round();
                        match state
                        {
                            [None, None] | [Some(_), Some(_)] => (),
                            [Some(p1), None] => 
                            {
                                if DEBUG { println!("DEBUG: hover, state = {:?}", state); }
                                painter.rect(rect_from_pos2(&p1, &p), Rounding::none(), Color32::from_white_alpha(30), Stroke::NONE); 
                            },
                            [None, Some(_)] => 
                            {
                                //ERRORE: per rimediare si resetta lo stato
                                if DEBUG {println!("DEBUG: error: state = [None, Some]")}
                                state = [None, None];
                            }
                        }
                    },
                    None => ()
                }
            } 
               
        });
    })
}