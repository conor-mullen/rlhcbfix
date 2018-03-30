use win;

#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "Windows error: {:?}", _0)]
    Windows(win::Error),
}

pub type HcbResult<T> = ::std::result::Result<T, Error>;

impl From<win::Error> for Error {
    fn from(err: win::Error) -> Error {
        Error::Windows(err)
    }
}
