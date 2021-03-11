use crate::configs::Configs;
use crate::id::Revision;

pub trait MechtronContext
{
    fn configs<'get>(&'get self) -> &'get Configs;
    fn revision(&self) -> &Revision;
}

