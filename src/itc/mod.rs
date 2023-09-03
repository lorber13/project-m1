use eframe::egui::Rect;
use std::path::PathBuf;


#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ScreenshotDim{ //Enum per la scelta del tipo di screenshot
    Fullscreen,
    Rectangle,
}

impl Clone for ScreenshotDim
{
    fn clone(&self) -> Self 
    {
        match self
        {
            ScreenshotDim::Fullscreen => ScreenshotDim::Fullscreen,
            ScreenshotDim::Rectangle => ScreenshotDim::Rectangle
        }
    }
}



pub enum SignalToHeadThread
{
    AcquirePressed(ScreenshotDim),
    RectSelected(Rect),
    PathSelected(PathBuf),
    Shutdown
}
