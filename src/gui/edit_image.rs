use eframe::egui::{CentralPanel, ColorImage, Context, Image, Widget};
use egui_extras::RetainedImage;
use image::RgbaImage;
use std::rc::Rc;

pub struct EditImage
{
    img: Image
}


pub enum EditImageEvent
{
    Saved, // todo: add image object to be returned
    Aborted,
    Nil
}

impl EditImage {
    pub fn new(img: Image) -> EditImage{
        EditImage {
            img,
        }
    }
    pub fn update(self: Rc<Self>, ctx: &Context, _frame: &mut eframe::Frame, enabled: bool) -> EditImageEvent {
        let mut ret = EditImageEvent::Nil;

        CentralPanel::default().show(ctx, |ui| {
            ui.add_enabled_ui(enabled, |ui1|{
                ui1.horizontal(|ui2|
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

                self.img.ui(ui1);
            });
        });
        ret
    }
}