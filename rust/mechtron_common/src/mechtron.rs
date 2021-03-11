use crate::configs::Configs;
use crate::id::{Revision, MechtronKey};
use crate::buffers::{Path, Buffer, ReadOnlyBuffer};
use crate::error::Error;
use crate::core::*;

#[derive(Clone)]
pub struct Context
{
    pub key: MechtronKey,
    pub cycle: i64,
    pub phase: String
}


impl Context
{
    pub fn to_buffer( configs: &Configs)->Result<Buffer,Error>
    {

        let path = Path::new(path![]);
        let buffer = {
            let factory = configs.schemas.get(&CORE_SCHEMA_MECHTRON_CONTEXT)?;
            let buffer = factory.new_buffer(Option::None)?;
            Buffer::new(buffer)
        };
        append( &path, &buffer )?;

        Ok(buffer)
    }

    pub fn from_buffer( buffer: &ReadOnlyBuffer, configs: &Configs )->Result<Context,Error>
    {
        let path = Path::new(path![]);
        Ok(Context::from(&path,buffer)?)
    }

    pub fn append(&self, path: &Path, buffer: &mut Buffer) -> Result<(), Error> {
        self.key.append(&path.push(path!("tron")), buffer)?;
        buffer.set( &path.with(path!["cycle"]), &self.cycle )?;
        buffer.set( &path.with(path!["phase"]), &self.phase )?;
        Ok(())
    }

    pub fn from(path: &Path, buffer: &ReadOnlyBuffer) -> Result<Self, Error> {
        let tron = MechtronKey::from(&path.push(path!("tron")), buffer)?;
        buffer.get( &path.with(path!["cycle"]))?;
        buffer.get( &path.with(path!["phase"]))?;
        Ok(Context{
            key: tron,
            cycle: cycle,
            phase: phase,
        })
    }
}