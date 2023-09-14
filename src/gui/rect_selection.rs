use crate::{image_coding, DEBUG};

use eframe::egui;
use eframe::egui::{Context, TextureHandle};
use egui::{pos2, Color32, ColorImage, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use image::RgbaImage;

pub struct RectSelection {
    texture_handle: TextureHandle,
    start_drag_point: Option<Pos2>,
    rgba: RgbaImage,
}

impl RectSelection {
    pub fn new(rgba: RgbaImage, ctx: &Context) -> Self {
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
            rgba,
            start_drag_point: None,
        }
    }

    pub fn update(
        &mut self,
        ctx: &Context,
        frame: &mut eframe::Frame,
    ) -> Option<(Rect, RgbaImage)> {
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
                        self.start_drag_point = space.hover_pos();
                    }
                    (false, true) => {
                        if let Some(pos1) = self.start_drag_point {
                            ret = Some(
                                (
                                    Rect::from_points(&[
                                        pos1,
                                        space
                                            .hover_pos()
                                            .expect("error"),
                                    ]),
                                    self.rgba.clone(),
                                ), // todo: ugly clone
                            );
                        }
                    }
                    (false, false) => {
                        if let Some(pos1) = self.start_drag_point {
                            painter.rect(
                                Rect::from_points(&[
                                    pos1,
                                    space.hover_pos().expect("error"),
                                ]),
                                Rounding::none(),
                                Color32::from_white_alpha(30), // todo: should be the opposite
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
