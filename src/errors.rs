use win;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Windows error: {:?}", _0)]
    Windows(#[cause] win::Error),
    #[fail(display = "No Rocket League process found.")]
    NoProcess,
}

pub type HcbResult<T> = ::std::result::Result<T, Error>;

impl From<win::Error> for Error {
    fn from(err: win::Error) -> Error {
        Error::Windows(err)
    }
}
