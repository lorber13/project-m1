use crate::{image_coding, DEBUG};

use crate::gui::edit_image::obscure_screen;
use eframe::egui;
use eframe::egui::{Context, CursorIcon, TextureHandle};
use egui::{pos2, Color32, ColorImage, Pos2, Rect, Rounding, Sense, Vec2};
use image::RgbaImage;

pub struct RectSelection {
    texture_handle: TextureHandle,
    start_drag_point: Option<Pos2>,
    rgba: RgbaImage,
}

impl RectSelection {
    pub fn new(rgba: RgbaImage, ctx: &Context) -> Self {
        if DEBUG {
            let _ = image_coding::start_thread_copy_to_clipboard(&rgba);
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

    pub fn update(&mut self, ctx: &Context) -> Option<(Rect, RgbaImage)> {
        let mut ret = None;

        egui::Area::new("area_1").show(ctx, |ui| {
            let (response, painter) = ui.allocate_painter(
                Vec2::new(ctx.screen_rect().width(), ctx.screen_rect().height()),
                Sense::click_and_drag(),
            );
            painter.image(
                self.texture_handle.id(),
                painter.clip_rect(),
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );

            ctx.set_cursor_icon(CursorIcon::Crosshair);
            if !response.clicked() {
                if response.drag_started() {
                    self.start_drag_point = response.hover_pos();
                    painter.rect_filled(
                        painter.clip_rect(),
                        Rounding::none(),
                        Color32::from_black_alpha(200),
                    );
                } else if response.dragged() {
                    if let Some(pos) = self.start_drag_point {
                        obscure_screen(
                            &painter,
                            Rect::from_points(&[pos, response.hover_pos().expect("error")]),
                        );
                    }
                } else if response.drag_released() {
                    if let Some(pos) = self.start_drag_point {
                        ret = Some(
                            (
                                // different displays have different pixels_per_point
                                Rect::from_points(&[
                                    pos2(
                                        pos.x * ctx.pixels_per_point(),
                                        pos.y * ctx.pixels_per_point(),
                                    ),
                                    response
                                        .hover_pos()
                                        .map(|pos| {
                                            pos2(
                                                pos.x * ctx.pixels_per_point(),
                                                pos.y * ctx.pixels_per_point(),
                                            )
                                        })
                                        .expect("error"),
                                ]),
                                self.rgba.clone(),
                            ), // todo: ugly clone
                        );
                    }
                } else {
                    painter.rect_filled(
                        painter.clip_rect(),
                        Rounding::none(),
                        Color32::from_black_alpha(200),
                    );
                }
            } else {
                painter.rect_filled(
                    painter.clip_rect(),
                    Rounding::none(),
                    Color32::from_black_alpha(200),
                );
            }
        });
        ret
    }
}
