#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ScreenshotDim{ //Enum per la scelta del tipo di screenshot
    Fullscreen,
    Rectangle,
}


pub enum SignalToHeadThread
{
    ShowMainWindow,
    AcquirePressed(ScreenshotDim),
    RectSelected(Rect),
    PathSelected(PathBuf)
}
