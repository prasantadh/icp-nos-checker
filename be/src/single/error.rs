pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Git(git2::Error),
    IO(std::io::Error),
    Utf(std::str::Utf8Error),
    ParseInt(std::num::ParseIntError),
    VarError(std::env::VarError, &'static str),
    Miscellaneous,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<git2::Error> for Error {
    fn from(value: git2::Error) -> Self {
        Self::Git(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(value: std::str::Utf8Error) -> Self {
        Self::Utf(value)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(value: std::num::ParseIntError) -> Self {
        Self::ParseInt(value)
    }
}
