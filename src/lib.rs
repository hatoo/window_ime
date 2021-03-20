#[cfg(target_os = "windows")]
#[path = "platform/windows.rs"]
mod platform;

use platform::IMEImpl;
use raw_window_handle::HasRawWindowHandle;
use std::error::Error;

pub struct IME {
    ime_impl: platform::IMEImpl,
}

impl IME {
    pub fn connect<W: HasRawWindowHandle>(window: &W) -> Result<Self, Box<dyn Error>> {
        let ime_impl = IMEImpl::new(window)?;

        Ok(IME { ime_impl })
    }

    pub fn set_position(&self, x: f32, y: f32) {
        self.ime_impl.set_position(x, y);
    }
}
