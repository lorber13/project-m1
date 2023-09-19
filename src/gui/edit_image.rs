use super::egui::ComboBox;
use crate::image_coding::ImageFormat;
use eframe::egui::{
    pos2, vec2, CentralPanel, Color32, ColorImage, Context, Painter, Rect, Response, Sense,
    TextureHandle, Ui,
};
use image::RgbaImage;

pub struct EditImage {
    image: RgbaImage,
    format: ImageFormat,
    texture_handle: TextureHandle,
}

pub enum EditImageEvent {
    Saved {
        image: RgbaImage,
        format: ImageFormat,
    },
    Aborted,
    Nil,
}

impl EditImage {
    pub fn new(rgba: RgbaImage, ctx: &Context) -> EditImage {
        EditImage {
            texture_handle: ctx.load_texture(
                "screenshot_image",
                ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize],
                    rgba.as_raw(),
                ),
                Default::default(),
            ),
            image: rgba,
            format: ImageFormat::Png,
        }
    }
    fn display_image(&mut self, ui: &mut Ui) -> (Response, Painter) {
        let available_size = ui.available_size_before_wrap();
        let image_size = self.texture_handle.size_vec2();
        let scaling_ratio = {
            let mut ratio = if image_size.x / available_size.x > image_size.y / available_size.y {
                available_size.x / image_size.x
            } else {
                available_size.y / image_size.y
            };
            if ratio > 1.0 {
                ratio = 1.0;
            }
            ratio
        };
        let scaled_dimensions = vec2(
            self.texture_handle.size()[0] as f32 * scaling_ratio,
            self.texture_handle.size()[1] as f32 * scaling_ratio,
        );
        let (response, painter) = ui.allocate_painter(scaled_dimensions, Sense::click_and_drag());
        painter.image(
            self.texture_handle.id(),
            Rect::from_min_size(painter.clip_rect().left_top(), scaled_dimensions),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        (response, painter)
    }
    pub fn update(
        &mut self,
        ctx: &Context,
        _frame: &mut eframe::Frame,
        enabled: bool,
    ) -> EditImageEvent {
        let mut ret = EditImageEvent::Nil;
        CentralPanel::default().show(ctx, |ui| {
            ui.add_enabled_ui(enabled, |ui| {
                ui.horizontal_top(|ui| {
                    ComboBox::from_label("") //menù a tendina per la scelta del formato di output
                        .selected_text(format!("{:?}", self.format))
                        .show_ui(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
                            ui.set_min_width(60.0);
                            ui.selectable_value(&mut self.format, ImageFormat::Png, "Png");
                            ui.selectable_value(&mut self.format, ImageFormat::JPEG, "Jpeg");
                            ui.selectable_value(&mut self.format, ImageFormat::GIF, "Gif");
                        });

                    if ui.button("Save").clicked() {
                        ret = EditImageEvent::Saved {
                            image: self.image.clone(),   // todo: ugly clone
                            format: self.format.clone(), // todo: should be a state
                        };
                    }

                    if ui.button("Abort").clicked() {
                        ret = EditImageEvent::Aborted;
                    }
                    ui.set_max_height(30.0);
                });
                ui.end_row();
                let (response, painter) = self.display_image(ui);
            });
        });
        ret
    }
}
