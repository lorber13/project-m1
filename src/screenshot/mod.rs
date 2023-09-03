use screenshots::Screen;
use egui_extras::RetainedImage;
use eframe::egui::ColorImage;


pub fn fullscreen_screenshot() -> RetainedImage {
    let shot = Screen::all().unwrap().first().unwrap().capture().unwrap(); // todo: modify in case of multiple monitors
    RetainedImage::from_color_image(
        "screenshot_image",
        ColorImage::from_rgba_unmultiplied([shot.width() as usize, shot.height() as usize], &shot),
    )
}

pub fn rect_screenshot() 
{
    //TO DO
}