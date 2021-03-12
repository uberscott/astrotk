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
    pub fn new( key: MechtronKey, cycle: i64, phase: String )->Self
    {
        Context{
            key: key,
            cycle: cycle,
            phase: phase
        }
    }

    pub fn to_bytes( &self, configs: &Configs)->Result<Vec<u8>,Error>
    {
        Ok(Buffer::bytes(self.to_buffer(configs)?))
    }

    pub fn to_buffer( &self, configs: &Configs)->Result<Buffer,Error>
    {

        let path = Path::new(path![]);
        let mut buffer = {
            let factory = configs.schemas.get(&CORE_SCHEMA_MECHTRON_CONTEXT)?;
            let buffer = factory.new_buffer(Option::None);
            Buffer::new(buffer)
        };
        self.append( &path, &mut buffer )?;

        Ok(buffer)
    }

    pub fn from_bytes( bytes: Vec<u8>, configs: &Configs )->Result<Context,Error>
    {
        let factory = configs.schemas.get(&CORE_SCHEMA_MECHTRON_CONTEXT)?;
        let buffer = factory.open_buffer(bytes);
        let buffer = Buffer::new(buffer).read_only();
        Context::from_buffer(&buffer,configs)
    }


    pub fn from_buffer( buffer: &ReadOnlyBuffer, configs: &Configs )->Result<Context,Error>
    {
        let path = Path::new(path![]);
        Ok(Context::from(&path,buffer)?)
    }

    pub fn append(&self, path: &Path, buffer: &mut Buffer) -> Result<(), Error> {
        self.key.append(&path.push(path!("tron")), buffer)?;
        buffer.set( &path.with(path!["cycle"]), self.cycle.clone() )?;
        buffer.set( &path.with(path!["phase"]), self.phase.clone() )?;
        Ok(())
    }

    pub fn from(path: &Path, buffer: &ReadOnlyBuffer) -> Result<Self, Error> {
        let tron = MechtronKey::from(&path.push(path!("tron")), buffer)?;
        let cycle = buffer.get( &path.with(path!["cycle"]))?;
        let phase = buffer.get( &path.with(path!["phase"]))?;
        Ok(Context{
            key: tron,
            cycle: cycle,
            phase: phase,
        })
    }
}