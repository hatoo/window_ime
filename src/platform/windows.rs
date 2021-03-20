#![allow(non_snake_case)]

use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct PlatformNotMatchedError;

impl fmt::Display for PlatformNotMatchedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Platform not matched")
    }
}

impl Error for PlatformNotMatchedError {}

pub struct IMEImpl {
    handle: raw_window_handle::windows::WindowsHandle,
}

impl IMEImpl {
    pub fn new<W: HasRawWindowHandle>(w: &W) -> Result<Self, Box<dyn Error>> {
        match w.raw_window_handle() {
            RawWindowHandle::Windows(handle) => Ok(Self { handle }),
            _ => Err(Box::new(PlatformNotMatchedError)),
        }
    }

    pub fn set_position(&self, x: f32, y: f32) {
        use std::mem;
        use winapi::shared::windef::POINT;
        use winapi::um::imm::{CFS_POINT, COMPOSITIONFORM};

        if unsafe { winuser::GetSystemMetrics(winuser::SM_IMMENABLED) } != 0 {
            let dpi = unsafe { hwnd_dpi(self.handle.hwnd as _) };
            let scale_factor = dpi_to_scale_factor(dpi) as f32;

            let mut composition_form = COMPOSITIONFORM {
                dwStyle: CFS_POINT,
                ptCurrentPos: POINT {
                    x: (x * scale_factor) as _,
                    y: (y * scale_factor) as _,
                },
                rcArea: unsafe { mem::zeroed() },
            };
            unsafe {
                let himc = winapi::um::imm::ImmGetContext(self.handle.hwnd as _);
                winapi::um::imm::ImmSetCompositionWindow(himc, &mut composition_form);
                winapi::um::imm::ImmReleaseContext(self.handle.hwnd as _, himc);
            }
        }
    }
}

use std::os::raw::c_void;

use winapi::{
    shared::{
        minwindef::{FALSE, UINT},
        ntdef::{HRESULT, LPCSTR},
        windef::{HMONITOR, HWND},
        winerror::S_OK,
    },
    um::{
        libloaderapi::{GetProcAddress, LoadLibraryA},
        shellscalingapi::{MDT_EFFECTIVE_DPI, MONITOR_DPI_TYPE},
        wingdi::{GetDeviceCaps, LOGPIXELSX},
        winuser::{self, MONITOR_DEFAULTTONEAREST},
    },
};

fn get_function_impl(library: &str, function: &str) -> Option<*const c_void> {
    assert_eq!(library.chars().last(), Some('\0'));
    assert_eq!(function.chars().last(), Some('\0'));

    // Library names we will use are ASCII so we can use the A version to avoid string conversion.
    let module = unsafe { LoadLibraryA(library.as_ptr() as LPCSTR) };
    if module.is_null() {
        return None;
    }

    let function_ptr = unsafe { GetProcAddress(module, function.as_ptr() as LPCSTR) };
    if function_ptr.is_null() {
        return None;
    }

    Some(function_ptr as _)
}

macro_rules! get_function {
    ($lib:expr, $func:ident) => {
        get_function_impl(concat!($lib, '\0'), concat!(stringify!($func), '\0'))
            .map(|f| unsafe { std::mem::transmute::<*const _, $func>(f) })
    };
}

pub type GetDpiForWindow = unsafe extern "system" fn(hwnd: HWND) -> UINT;
pub type GetDpiForMonitor = unsafe extern "system" fn(
    hmonitor: HMONITOR,
    dpi_type: MONITOR_DPI_TYPE,
    dpi_x: *mut UINT,
    dpi_y: *mut UINT,
) -> HRESULT;

lazy_static::lazy_static! {
    pub static ref GET_DPI_FOR_WINDOW: Option<GetDpiForWindow> =
        get_function!("user32.dll", GetDpiForWindow);
    pub static ref GET_DPI_FOR_MONITOR: Option<GetDpiForMonitor> =
        get_function!("shcore.dll", GetDpiForMonitor);
}

const BASE_DPI: u32 = 96;
fn dpi_to_scale_factor(dpi: u32) -> f64 {
    dpi as f64 / BASE_DPI as f64
}

unsafe fn hwnd_dpi(hwnd: HWND) -> u32 {
    let hdc = winuser::GetDC(hwnd);
    if hdc.is_null() {
        panic!("[winit] `GetDC` returned null!");
    }
    if let Some(GetDpiForWindow) = *GET_DPI_FOR_WINDOW {
        // We are on Windows 10 Anniversary Update (1607) or later.
        match GetDpiForWindow(hwnd) {
            0 => BASE_DPI, // 0 is returned if hwnd is invalid
            dpi => dpi as u32,
        }
    } else if let Some(GetDpiForMonitor) = *GET_DPI_FOR_MONITOR {
        // We are on Windows 8.1 or later.
        let monitor = winuser::MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if monitor.is_null() {
            return BASE_DPI;
        }

        let mut dpi_x = 0;
        let mut dpi_y = 0;
        if GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) == S_OK {
            dpi_x as u32
        } else {
            BASE_DPI
        }
    } else {
        // We are on Vista or later.
        if winuser::IsProcessDPIAware() != FALSE {
            // If the process is DPI aware, then scaling must be handled by the application using
            // this DPI value.
            GetDeviceCaps(hdc, LOGPIXELSX) as u32
        } else {
            // If the process is DPI unaware, then scaling is performed by the OS; we thus return
            // 96 (scale factor 1.0) to prevent the window from being re-scaled by both the
            // application and the WM.
            BASE_DPI
        }
    }
}
