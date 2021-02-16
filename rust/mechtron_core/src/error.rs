use std::error::Error;
use std::fmt;
use std::fmt::Formatter;
use no_proto::error::NP_Error;
use std::sync::{PoisonError, RwLockWriteGuard};

#[derive(Debug, Clone)]
pub struct MechError {}

impl fmt::Display for MechError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "a mech error occurred")
    }
}

impl From<NP_Error> for MechError{
    fn from(_: NP_Error) -> Self {
        MechError{}
    }
}

impl From<Box<dyn Error>> for MechError{
    fn from(_: Box<dyn Error>) -> Self {
        MechError{}
    }
}


