use no_proto::buffer::NP_Buffer;
use no_proto::NP_Factory;

use crate::artifact::Artifact;
use std::error::Error;
use no_proto::memory::{NP_Memory_Owned, NP_Memory, NP_Mem_New};
use no_proto::pointer::{NP_Scalar, NP_Value};
use std::sync::Arc;
use no_proto::error::NP_Error;

pub trait BufferFactories
{
    fn create_buffer(&self, artifact: &Artifact) ->Result<NP_Buffer<NP_Memory_Owned>,Box<dyn Error>>;
    fn create_buffer_from_array(&self, artifact: &Artifact, array: Vec<u8> ) ->Result<NP_Buffer<NP_Memory_Owned>,Box<dyn Error>>;
    fn create_buffer_from_buffer(&self, artifact: &Artifact, buffer: NP_Buffer<NP_Memory_Owned> ) ->Result<NP_Buffer<NP_Memory_Owned>,Box<dyn Error>>;
    fn get_buffer_factory(&self, artifact: &Artifact) ->Option<&'static NP_Factory<'static>>;
}


pub fn get<'get, X: 'get,M: NP_Memory + Clone + NP_Mem_New>(buffer:&'get NP_Buffer<M>, path: &[&str]) -> Result<X, Box<dyn Error>> where X: NP_Value<'get> + NP_Scalar<'get> {
    match buffer.get::<X>(path)
    {
        Ok(option)=>{
            match option{
                Some(rtn)=>Ok(rtn),
                None=>Err(format!("expected a value for {}", path[path.len()-1] ).into())
            }
        },
        Err(e)=>Err(format!("could not get {}",cat(path)).into())
    }
}



pub fn set<'get, X: 'get,M: NP_Memory + Clone + NP_Mem_New>(buffer:&'get mut NP_Buffer<M>, path: &[&str], value: X) -> Result<bool, Box<dyn Error>> where X: NP_Value<'get> + NP_Scalar<'get> {
    match buffer.set::<X>(path, value)
    {
        Ok(option)=>{
            match option{
                Some(rtn)=>Ok(rtn),
                None=>Err(format!("expected a value for {}", path[path.len()-1] ).into())
            }
        },
        Err(e)=>Err(format!("could not set {}",cat(path)).into())
    }
}

#[derive(Clone)]
pub struct Buffer
{
    np_buffer: NP_Buffer<NP_Memory_Owned>
}

impl Buffer
{
    pub fn new( np_buffer: NP_Buffer<NP_Memory_Owned> )->Self
    {
        Buffer{
            np_buffer: np_buffer
        }
    }

    pub fn get<'get, X: 'get>( &self, path: &[&str]) -> Result<X, Box<dyn Error>> where X: NP_Value<'get> + NP_Scalar<'get> {
        match self.buffer.get::<X>(path)
        {
            Ok(option)=>{
                match option{
                    Some(rtn)=>Ok(rtn),
                    None=>Err(format!("expected a value for {}", path[path.len()-1] ).into())
                }
            },
            Err(e)=>Err(format!("could not get {}",cat(path)).into())
        }
    }

    pub fn set<'get, X: 'get>(&mut self, path: &[&str], value: X) -> Result<bool, Box<dyn Error>> where X: NP_Value<'get> + NP_Scalar<'get> {
        match self.buffer.set::<X>(path, value)
        {
            Ok(option)=>{
                match option{
                    Some(rtn)=>Ok(rtn),
                    None=>Err(format!("expected a value for {}", path[path.len()-1] ).into())
                }
            },
            Err(e)=>Err(format!("could not set {}",cat(path)).into())
        }
    }

    pub fn compact( &mut self )->Result<(),Box<dyn Error>>
    {
        match self.np_buffer.compact(Option::None)
        {
            Ok(_) => Ok(()),
            Err(e) => Err("could not compact".into() )
        }
    }

    pub fn read_only(buffer: Buffer ) ->RO_Buffer
    {
        RO_Buffer {
            np_buffer: buffer.np_buffer
        }
    }
}


#[derive(Clone)]
pub struct RO_Buffer
{
    np_buffer: NP_Buffer<NP_Memory_Owned>
}

impl RO_Buffer
{

    pub fn new( np_buffer: NP_Buffer<NP_Memory_Owned> )->Self
    {
        RO_Buffer{
            np_buffer: np_buffer
        }
    }

    pub fn get<'get, X: 'get>( &self, path: &[&str]) -> Result<X, Box<dyn Error>> where X: NP_Value<'get> + NP_Scalar<'get> {
        match self.buffer.get::<X>(path)
        {
            Ok(option)=>{
                match option{
                    Some(rtn)=>Ok(rtn),
                    None=>Err(format!("expected a value for {}", path[path.len()-1] ).into())
                }
            },
            Err(e)=>Err(format!("could not get {}",cat(path)).into())
        }
    }

    pub fn clone_to_buffer( &self )->Result<Buffer,Box<dyn Error>>
    {
        Ok(Buffer {
            np_buffer: self.np_buffer.clone()
        })
    }
}
