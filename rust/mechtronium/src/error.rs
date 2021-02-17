use wasmer::CompileError;
use std::fmt::Formatter;
use core::fmt;

#[derive(Debug, Clone)]
pub struct Error{
    pub error: String
}

impl fmt::Display for Error{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "nucleus error: {:?}",self)
    }
}

impl From<mechtron_core::error::Error> for Error {

    fn from(e:mechtron_core::error::Error) -> Self {
        Error {
            error: format!("{:?}", e)
        }
    }
}
