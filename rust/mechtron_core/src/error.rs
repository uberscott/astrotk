use std::{fmt, io};
use std::fmt::{Formatter, Debug};
use no_proto::error::NP_Error;
use std::sync::{PoisonError, RwLockWriteGuard};
use std::time::SystemTimeError;
use semver::SemVerError;
use std::string::FromUtf8Error;


#[derive(Debug, Clone)]
pub struct Error{
    pub error: String
}

impl fmt::Display for Error{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "nucleus error: {:?}",self)
    }
}

impl From<NP_Error> for Error{
    fn from(e: NP_Error) -> Self {
        Error{
            error: format!("{:?}",e)
        }
    }
}

impl From<io::Error> for Error{
    fn from(e: io::Error) -> Self {
        Error{
            error: format!("{:?}",e)
        }
    }
}

impl From<Box<dyn std::error::Error>> for Error{
    fn from(e: Box<dyn std::error::Error>) -> Self {
        Error{
            error: format!("{:?}",e)
        }
    }
}

impl From<&dyn std::error::Error> for Error{
    fn from(e: &dyn std::error::Error) -> Self {
        Error{
            error: format!("{:?}",e)
        }
    }
}

impl From<Box<dyn Debug>> for Error{
    fn from(e: Box<Debug>) -> Self {
        Error{
            error: format!("{:?}",e)
        }
    }
}

impl From<&dyn Debug> for Error{
    fn from(e: &dyn Debug) -> Self {
        Error{
            error: format!("{:?}",e)
        }
    }
}


impl From<&str> for Error{
    fn from(e: &str) -> Self {
        Error{
            error: format!("{:?}",e)
        }
    }
}

impl From<String> for Error{
    fn from(e: String) -> Self {
        Error{
            error: format!("{:?}",e)
        }
    }
}

impl<T> From<PoisonError<T>> for Error {
    fn from(e: PoisonError<T>) -> Self {
        Error {
            error: format!("{:?}", e)
        }
    }
}
impl From<SystemTimeError> for Error {
    fn from(e: SystemTimeError) -> Self {
        Error {
            error: format!("{:?}", e)
        }
    }
}

impl From<SemVerError> for Error {
    fn from(e: SemVerError) -> Self {
        Error {
            error: format!("{:?}", e)
        }
    }
}

impl From<serde_yaml::Error> for Error {

    fn from(e: serde_yaml::Error) -> Self {
        Error {
            error: format!("{:?}", e)
        }
    }
}
impl From<FromUtf8Error> for Error {

    fn from(e:FromUtf8Error) -> Self {
        Error {
            error: format!("{:?}", e)
        }
    }
}

