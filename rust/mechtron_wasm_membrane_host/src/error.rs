use wasmer::{InstantiationError, RuntimeError, ExportError, CompileError};

#[derive(Debug)]
pub struct Error
{
    error: String
}

impl From<Box<dyn std::error::Error>> for Error{
    fn from(e: Box<dyn std::error::Error>) -> Self {
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

impl From<&str> for Error{
    fn from(e: &str) -> Self {
        Error{
            error: format!("{:?}",e)
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

impl From<std::io::Error> for Error{
    fn from(e: std::io::Error) -> Self {
        Error{
            error: format!("{:?}",e)
        }
    }
}

/*
impl From<Box<NoneError>> for Error{
    fn from(e: Box<NoneError>) -> Self {
        Error{
            error: format!("{:?}",e)
        }
    }
}

 */
