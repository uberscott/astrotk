use crate::configs::Configs;
use crate::id::Revision;

pub trait MechtronContext<'cycle>
{
    fn configs<'get>(&'get self) -> &'get Configs<'cycle>;
    fn revision(&self) -> &Revision;
}

