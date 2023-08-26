use eframe::egui;
use egui::{pos2, Color32, ColorImage, Pos2, Rect, Rounding, Sense, Stroke, Vec2, CentralPanel, Key};
use egui_extras::RetainedImage;
use screenshots::Screen;


fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        //fullscreen: true,
        ..Default::default()
    };
    eframe::run_native(
        "Rect_selection_revisited",
        options,
        Box::new(|_cc| Box::new(MyApp::new())),
    )
}

struct MyApp {
    capturing: bool,
    image: RetainedImage,
    state: [Option<Pos2>; 2],
}

impl MyApp {
    fn new() -> Self {
        Self {
            capturing: false,
            image: RetainedImage::from_color_image("todo", ColorImage::default()),
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
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.capturing {
            frame.set_fullscreen(true);
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
                    if ctx.input(|i| {i.key_pressed(Key::Enter)}) {
                        println!("salvo lo screenshot ritagliato");
                        self.capturing = false;
                        self.state = [None, None];
                        frame.set_fullscreen(false);
                        frame.set_maximized(true);
                    }
                }


                if space.drag_started() && !space.drag_released()
                //se inizio del drag, si memorizzano le coordinate del puntatore
                {
                    self.state = [ space.hover_pos().and_then(|point| { Some(point.round()) }), None ];
                } else if space.drag_released()
                //se è terminato il drag, si memorizza la posizione del puntatore
                {
                    self.state[1] = space.hover_pos().and_then(|point| { Some(point.round())});
                } else
                //durante il drag & drop (quindi, solo se lo stato contiene già il primo punto), si disegna il rettangolo
                {
                    match space.hover_pos() {
                        Some(p_not_round) => {
                            let p = p_not_round.round();
                            match self.state {
                                [None, None] | [Some(_), Some(_)] => (),
                                [Some(p1), None] => {
                                    painter.rect(
                                        rect_from_pos2(&p1, &p),
                                        Rounding::none(),
                                        Color32::from_white_alpha(30),
                                        Stroke::NONE,
                                    );
                                }
                                [None, Some(_)] => {
                                    //ERRORE: per rimediare si resetta lo stato
                                    self.state = [None, None];
                                }
                            }
                        }
                        None => (),
                    }
                }
            });

        } else {
            CentralPanel::default().show(ctx, |ui| {
                ui.label("premendo il pulsante invio si salva lo screenshot (per ora equivale a una println)");
                if ui.button("capture").clicked() {
                    self.capturing = true;
                    self.image = capture_screenshot();
                }
            });
        }
    }
}

fn capture_screenshot() -> RetainedImage {
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
    image
}
