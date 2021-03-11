use wasmer::{CompileError, RuntimeError, ExportError, InstantiationError};
use std::fmt::{Formatter, Debug};
use core::fmt;
use std::string::FromUtf8Error;
use semver::SemVerError;
use std::time::SystemTimeError;
use std::sync::PoisonError;
use no_proto::error::NP_Error;
use std::io;

#[derive(Debug, Clone)]
pub struct Error{
    pub error: String
}

impl Error
{
    pub fn downgrade<R>( result: Result<R,Error>)->Result<R, mechtron_common::error::Error>
    {
        match result{
            Ok(ok) => Ok(ok),
            Err(error) => Err(
                error.error.into()
            )
        }

    }
}


impl fmt::Display for Error{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "nucleus error: {:?}",self)
    }
}

impl From<mechtron_common::error::Error> for Error {

    fn from(e: mechtron_common::error::Error) -> Self {
        Error {
            error: format!("{:?}", e)
        }
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

impl From<RuntimeError> for Error{
    fn from(e: RuntimeError) -> Self {
        Error{
            error: format!("{:?}",e)
        }
    }
}

impl From<ExportError> for Error{
    fn from(e: ExportError) -> Self {
        Error{
            error: format!("{:?}",e)
        }
    }
}

impl From<CompileError> for Error{
    fn from(e: CompileError) -> Self {
        Error{
            error: format!("{:?}",e)
        }
    }
}

impl From<InstantiationError> for Error{
    fn from(e: InstantiationError) -> Self {
        Error{
            error: format!("{:?}",e)
        }
    }
}