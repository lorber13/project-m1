


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

pub enum SettingsEvent
{
    Saved,
    Aborted,
    Nil,
    Error(&'static str)
}

#[derive(Clone, Copy)]
pub struct Delay {
    pub delayed: bool,
    pub scalar: f64,
}

