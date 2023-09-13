use crate::{image_coding, DEBUG};

use eframe::egui;
use eframe::egui::TextureHandle;
use egui::{pos2, Color32, ColorImage, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use image::RgbaImage;

pub struct RectSelection {
    texture_handle: TextureHandle,
    image: RgbaImage,
    start_drag_point: Option<Pos2>,
}

impl RectSelection {
    pub fn new(rgba: RgbaImage, ctx: &egui::Context) -> Self {
        if DEBUG {
            let _ = image_coding::copy_to_clipboard(&rgba);
        }
        RectSelection {
            texture_handle: ctx.load_texture(
                "screenshot_image",
                ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize],
                    rgba.as_raw(),
                ),
                Default::default(),
            ),
            image: rgba,
            start_drag_point: None,
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) -> Option<RgbaImage> {
        frame.set_fullscreen(true); // todo: should be called once, not every frame

        let mut ret = None;

        egui::Area::new("area_1").show(ctx, |ui| {
            let (space, painter) = ui.allocate_painter(
                Vec2::new(ctx.screen_rect().width(), ctx.screen_rect().height()),
                Sense::click_and_drag(),
            );
            painter.image(
                self.texture_handle.id(),
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
                        let rect = Rect::from_points(&[
                            self.start_drag_point.expect(
                                "if we are in this state, start_drag_point must be defined",
                            ),
                            space
                                .hover_pos()
                                .map(|point| point.round())
                                .expect("same here"),
                        ]);
                        ret = Some(
                            image::imageops::crop(
                                &mut self.image,
                                rect.left_top().x as u32,
                                rect.left_top().y as u32,
                                rect.width() as u32,
                                rect.height() as u32,
                            )
                            .to_image(),
                        );
                    }
                    (false, false) => {
                        if let Some(pos1) = self.start_drag_point {
                            painter.rect(
                                Rect::from_points(&[
                                    pos1,
                                    space
                                        .hover_pos()
                                        .map(|point| point.round())
                                        .expect("same here"),
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
        ret
    }
}
