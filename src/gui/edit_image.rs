use eframe::egui::{CentralPanel, ColorImage, Context, Image, Widget};
use egui_extras::RetainedImage;
use image::RgbaImage;

pub struct EditImage
{
    img: Image
}

enum EditImageEvent
{
    Saved, // todo: add image object to be returned
    Aborted,
    Nil
}

impl EditImage {
    pub fn new() {
        EditImage {
            img: (),
        }
    }
    pub fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) -> EditImageEvent {
        let mut ret = EditImageEvent::Nil;

        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui2|
                    {
                        if ui2.button("Save").clicked()
                        {
                            ret = EditImageEvent::Saved;
                        }else if ui2.button("Abort").clicked()
                        {
                            ret = EditImageEvent::Aborted;
                        }
                    }
            );
            self.img.ui(ui);
        });
        ret
    }
}