use crate::image_coding::ImageFormat;
use eframe::egui::{CentralPanel, ColorImage, Context, Sense, TextureHandle};
use image::RgbaImage;

pub struct EditImage {
    image: RgbaImage,
    texture_handle: TextureHandle,
}

pub enum EditImageEvent {
    Saved {
        image: RgbaImage,
        format: ImageFormat,
    }, // todo: add image object to be returned
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
        }
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
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        ret = EditImageEvent::Saved {
                            image: self.image.clone(), // todo: ugly clone
                            format: ImageFormat::Png,  // todo: should be a state
                        };
                    } else if ui.button("Abort").clicked() {
                        ret = EditImageEvent::Aborted;
                    }
                });
                let (response, painter) =
                    ui.allocate_painter(self.texture_handle.size_vec2(), Sense::click_and_drag());
                ui.image(self.texture_handle.id(), self.texture_handle.size_vec2());
            });
        });
        ret
    }
}
