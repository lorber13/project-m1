use crate::{image_coding, DEBUG};

use eframe::egui;
use egui::{pos2, Color32, ColorImage, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use egui_extras::RetainedImage;
use image::RgbaImage;

pub struct RectSelection {
    image: RetainedImage,
    start_drag_point: Option<Pos2>,
}

impl RectSelection {
    pub fn new(rgba: &RgbaImage) -> Self {
        if DEBUG {
            let _ = image_coding::copy_to_clipboard(&rgba);
        }
        let image = RetainedImage::from_color_image(
            "screenshot_image",
            ColorImage::from_rgba_unmultiplied(
                [rgba.width() as usize, rgba.height() as usize],
                &rgba,
            ),
        );
        Self {
            image,
            start_drag_point: None,
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) -> Option<RgbaImage> {
        frame.set_fullscreen(true); // todo: should be called once, not every frame

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
                        todo!()
                        //return Some(Rect::from_points(&[
                        //    self.start_drag_point.unwrap(),
                        //    space.hover_pos().map(|point| point.round()).expect("error"),
                        //]));
                    }
                    (false, false) => {
                        if let Some(pos1) = self.start_drag_point {
                            painter.rect(
                                Rect::from_points(&[
                                    pos1,
                                    space.hover_pos().map(|point| point.round()).expect("error"),
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
        None
    }
}
