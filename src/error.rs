use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Bincode(bincode::Error),
    Io(std::io::Error),
    Reqwest(reqwest::Error),
    LogDirectory,
    NoRecentLog,
}

impl From<bincode::Error> for Error {
    fn from(e: bincode::Error) -> Self {
        Self::Bincode(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Reqwest(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Bincode(e) => write!(f, "{}", e),
            Self::Io(e) => write!(f, "{}", e),
            Self::Reqwest(e) => write!(f, "{}", e),
            Self::LogDirectory => write!(f, "log directory error"),
            Self::NoRecentLog => write!(f, "unable to find recent log"),
        }
    }
}
