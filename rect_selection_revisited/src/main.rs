use eframe::egui;
use egui::{pos2, Color32, ColorImage, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use egui_extras::RetainedImage;
use screenshots::Screen;

const DEBUG: bool = false; //if true, it prints messages on console

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        fullscreen: true,
        ..Default::default()
    };
    eframe::run_native(
        "Rect_selection_revisited",
        options,
        Box::new(|_cc| Box::new(MyApp::new())),
    )
}

struct MyApp {
    image: RetainedImage,
    state: [Option<Pos2>; 2],
}

impl MyApp {
    fn new() -> Self {
        let shot = Screen::all()
            .unwrap()
            .iter()
            .next()
            .unwrap()
            .capture()
            .unwrap(); // da modificare in caso di monitor multipli
        let image = RetainedImage::from_color_image(
            "screenshot_image",
            ColorImage::from_rgba_unmultiplied(
                [shot.width() as usize, shot.height() as usize],
                &shot,
            ),
        );
        Self {
            image,
            state: [None, None],
        }
    }
}

fn rect_from_pos2(p1: &Pos2, p2: &Pos2) -> Rect {
    let min_x = if p1.x < p2.x { p1.x } else { p2.x };
    let min_y = if p1.y < p2.y { p1.y } else { p2.y };
    let left_top = Pos2::new(min_x.round(), min_y.round());
    let max_x = if p1.x > p2.x { p1.x } else { p2.x };
    let max_y = if p1.y > p2.y { p1.y } else { p2.y };
    let right_bottom = Pos2::new(max_x.round(), max_y.round());

    Rect {
        min: left_top,
        max: right_bottom,
    }
}
impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::Area::new("area_1").show(ctx, |ui| {
            let (space, painter) = ui.allocate_painter(
                Vec2::new(ctx.screen_rect().width(), ctx.screen_rect().height()),
                Sense::click_and_drag(),
            );
            painter.image(
                self.image.texture_id(ctx),
                Rect::from_min_max(
                    pos2(0.0, 0.0),
                    pos2(ctx.screen_rect().width(), ctx.screen_rect().height()),
                ),
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::from_white_alpha(30),
            );
            if let [Some(p1), Some(p2)] = self.state {
                painter.rect(
                    rect_from_pos2(&p1, &p2),
                    Rounding::none(),
                    Color32::from_white_alpha(10),
                    Stroke::NONE,
                );
            }

            if space.clicked()
            //se è avvenuto solo click e non drag, si resetta lo stato
            {
                self.state = [None, None];
                if DEBUG {
                    println!("state = {:?}", self.state);
                }
            } else if space.drag_started() && !space.drag_released()
            //se inizio del drag, si memorizzano le coordinate del puntatore
            {
                match space.hover_pos() {
                    None => (),
                    Some(p_not_round) => {
                        let p = p_not_round.round();
                        match self.state {
                            [None, None] | [Some(_), None] => {
                                self.state = [Some(p), None];
                                if DEBUG {
                                    println!("state = {:?}", self.state);
                                }
                            }
                            [None, Some(_)] => {
                                //ERRORE: per rimediare si resetta lo stato
                                if DEBUG {
                                    println!("DEBUG: error: state = [None, Some]")
                                }
                                self.state = [None, None];
                            }
                            [Some(_), Some(_)] => {
                                //si riavvia un nuovo drag dopo averne terminato uno
                                self.state = [Some(p), None];
                            }
                        }
                    }
                }
            } else if space.drag_released()
            //se è terminato il drag, si memorizza la posizione del puntatore
            {
                if let Some(p_not_round) = space.hover_pos() {
                    let p2 = p_not_round.round();
                    match self.state {
                        [Some(p1), None] => {
                            //se il drag è stato rilasciato, necessariamente lo stato deve contenere già un punto
                            self.state = [Some(p1), Some(p2)];
                            if DEBUG {
                                println!("state = {:?}", self.state);
                            }
                        }
                        _ => {
                            //ERRORE: per rimediare si resetta lo stato
                            if DEBUG {
                                println!("DEBUG: error: state = [None, Some]")
                            }
                            self.state = [None, None];
                        }
                    }
                }
            } else
            //durante il drag & drop (quindi, solo se lo stato contiene già il primo punto), si disegna il rettangolo
            {
                match space.hover_pos() {
                    Some(p_not_round) => {
                        let p = p_not_round.round();
                        match self.state {
                            [None, None] | [Some(_), Some(_)] => (),
                            [Some(p1), None] => {
                                if DEBUG {
                                    println!("DEBUG: hover, state = {:?}", self.state);
                                }
                                painter.rect(
                                    rect_from_pos2(&p1, &p),
                                    Rounding::none(),
                                    Color32::from_white_alpha(30),
                                    Stroke::NONE,
                                );
                            }
                            [None, Some(_)] => {
                                //ERRORE: per rimediare si resetta lo stato
                                if DEBUG {
                                    println!("DEBUG: error: state = [None, Some]")
                                }
                                self.state = [None, None];
                            }
                        }
                    }
                    None => (),
                }
            }
        });
    }
}
