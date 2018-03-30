use winapi::shared::minwindef::DWORD;
use winapi::um::errhandlingapi::GetLastError;

#[derive(Clone, Copy, Debug)]
pub struct Error(DWORD);

impl Error {
    pub fn code(&self) -> u32 {
        self.0
    }

    /// Returns the last windows error.
    pub fn last() -> Error {
        Error(unsafe { GetLastError() })
    }
}

pub type WinResult<T> = ::std::result::Result<T, Error>;
