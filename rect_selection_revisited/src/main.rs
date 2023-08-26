use eframe::egui;
use egui::{pos2, CentralPanel, Color32, ColorImage, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use egui_extras::RetainedImage;
use screenshots::Screen;

fn main() -> Result<(), eframe::Error> {
    eframe::run_native(
        "Rect_selection_revisited",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Box::new(MyApp::new())),
    )
}

struct MyApp {
    capturing: bool,
    image: RetainedImage,
    start_drag_point: Option<Pos2>,
}

impl MyApp {
    fn new() -> Self {
        Self {
            capturing: false,
            image: RetainedImage::from_color_image("todo", ColorImage::default()),
            start_drag_point: None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.capturing {
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

                if !space.clicked() {
                    match (space.drag_started(), space.drag_released()) {
                        (true, false) => {
                            self.start_drag_point = space.hover_pos().map(|point| point.round());
                        }
                        (false, true) => {
                            println!("salvo lo screenshot ritagliato");
                            self.capturing = false;
                            self.start_drag_point = None;
                            frame.set_fullscreen(false);
                        }
                        (false, false) => {
                            if let Some(pos1) = self.start_drag_point {
                                painter.rect(
                                    Rect::from_points(&[
                                        pos1,
                                        space
                                            .hover_pos()
                                            .map(|point| point.round())
                                            .expect("errore"),
                                    ]),
                                    Rounding::none(),
                                    Color32::from_white_alpha(30),
                                    Stroke::NONE,
                                )
                            }
                        }
                        _ => {}
                    }
                } else {
                    // line put to prevent a strange bug in case of a click todo: investigate
                    self.start_drag_point = None;
                }
            });
        } else {
            CentralPanel::default().show(ctx, |ui| {
                ui.label("Non serve piu' invio per salvare il crop");
                if ui.button("capture").clicked() {
                    self.capturing = true;
                    self.image = capture_screenshot();
                    frame.set_fullscreen(true);
                }
            });
        }
    }
}

fn capture_screenshot() -> RetainedImage {
    let shot = Screen::all().unwrap().first().unwrap().capture().unwrap(); // da modificare in caso di monitor multipli
    RetainedImage::from_color_image(
        "screenshot_image",
        ColorImage::from_rgba_unmultiplied([shot.width() as usize, shot.height() as usize], &shot),
    )
}
