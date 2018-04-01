use std::fmt;

use winapi::shared::minwindef::DWORD;
use winapi::um::errhandlingapi::GetLastError;

#[derive(Debug, Copy, Clone, Fail)]
pub struct Error(DWORD);

impl Error {
    pub fn code(&self) -> u32 {
        self.0
    }

    pub fn description(&self) -> Option<&'static str> {
        match self.0 {
            31 => Some("This device is not working properly because Windows cannot load the drivers required for this device."),
            _ => None
        }
    }

    /// Returns the last windows error.
    pub fn last() -> Error {
        Error(unsafe { GetLastError() })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if let Some(desc) = self.description() {
            write!(f, "Windows error {}: {}", self.0, desc)
        } else {
            write!(f, "Windows error {}", self.0)
        }
    }
}

pub type WinResult<T> = ::std::result::Result<T, Error>;
