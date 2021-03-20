use raw_window_handle::HasRawWindowHandle;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct PlatformNotSupportedError;

impl fmt::Display for PlatformNotSupportedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Platform not supported")
    }
}

impl Error for PlatformNotSupportedError {}
pub struct IMEImpl {}

impl IMEImpl {
    pub fn new<W: HasRawWindowHandle>(_w: &W) -> Result<Self, Box<dyn Error>> {
        Err(Box::new(PlatformNotSupportedError))
    }

    pub fn set_position(&self, _x: f32, _y: f32) {
        unimplemented!();
    }
}
