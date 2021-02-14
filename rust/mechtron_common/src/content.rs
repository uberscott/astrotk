use crate::id::ContentKey;
use no_proto::buffer::{NP_Buffer, NP_Finished_Buffer};
use std::sync::Arc;
use no_proto::memory::{NP_Memory_Owned, NP_Memory_Ref};
use crate::artifact::Artifact;
use no_proto::NP_Factory;
use crate::buffers::{BufferFactories, Buffer, RO_Buffer};
use crate::configs::{Keeper, Configs};
use crate::configs::CORE;
use crate::configs::CORE_CONTENT_META;
use no_proto::error::NP_Error;
use std::error::Error;
use crate::message::Payload;


#[derive(Clone)]
pub struct Content
{
    pub meta: Buffer,
    pub data: Buffer
}

impl Content
{
    pub fn new(  configs: &Configs, artifact: Artifact )->Result<Self,Box<dyn Error+'_>>
    {
        let data = Buffer::new(configs.buffer_factory_keeper.get(&artifact).unwrap().new_buffer(Option::None));
        let meta=Buffer::new(configs.buffer_factory_keeper.get(&CORE_CONTENT_META)?.new_buffer(Option::None));
        Ok(Content{
            meta:meta,
            data:data
        })
    }

    pub fn from( artifact:Artifact, meta: NP_Buffer<NP_Memory_Owned>, data: NP_Buffer<NP_Memory_Owned> ) -> Self
    {
        Content {
            meta: Buffer::new(meta),
            data: Buffer::new(data)
        }
    }

    pub fn read_only( content: Content ) -> Result<ReadOnlyContent,Box<dyn Error>>
    {
        Ok(ReadOnlyContent{
            meta: Buffer::read_only(content.meta ),
            data: Buffer::read_only(content.data )
        })
    }

    pub fn compact( &mut self ) -> Result<(),Box<dyn Error>>
    {
        self.meta.compact()?;
        self.data.compact()?;

        Ok(())
    }


}

impl ContentMeta for Content
{
    fn set_artifact(&mut self, artifact: &Artifact) -> Result<bool, Box<dyn Error>> {
        Ok(self.meta.set(&path!["artifact"], artifact.to() )?)
    }

    fn set_creation_timestamp(&mut self, value: i64) -> Result<bool, Box<dyn Error>> {
        Ok(self.meta.set(&path!["creation_timestamp"], value  )?)
    }

    fn set_creation_cycle(&mut self, value: i64) -> Result<bool, Box<dyn Error>> {
        Ok(self.meta.set(&path!["creation_cycle"], value  )?)
    }
}

impl ReadOnlyContentMeta for Content
{
    fn get_artifact(&self) -> Result<Artifact, Box<dyn Error>> {
        Ok(Artifact::from(self.meta.get(&path!["artifact"] )?)?)
    }

    fn get_creation_timestamp(&self) -> Result<i64, Box<dyn Error>> {
        Ok(self.meta.get(&path!["creation_timestamp"] )?)
    }

    fn get_creation_cycle(&self) -> Result<i64, Box<dyn Error>> {
        Ok(self.meta.get(&path!["creation_cycle"] )?)
    }
}


pub struct ReadOnlyContent
{
    pub meta: RO_Buffer,
    pub data: RO_Buffer
}

impl ReadOnlyContent
{
    pub fn copy( &self )->Result<Content,Box<dyn Error>>
    {
        Ok(Content {
            meta: self.meta.clone_to_buffer()?,
            data: self.data.clone_to_buffer()?,
        })
    }

    // sucks that I have to clone the buffers here...
    pub fn payloads(&self)->Result<Vec<Payload>,Box<dyn Error>>
    {
        let rtn : Vec<Payload> = vec![
            Payload{
                buffer: self.meta.clone(),
                artifact: CORE_CONTENT_META.clone()
            },
            Payload{
                buffer: self.data.clone(),
                artifact: self.get_artifact()?
            }
        ];

        return Ok(rtn);
    }

    pub fn convert_to_payloads( configs: &Configs, content: ReadOnlyContent ) -> Result<Vec<Payload>,Box<dyn Error>>{

        let artifact = content.get_artifact()?;
        let rtn : Vec<Payload> = vec![
            Payload{
                buffer: content.meta,
                artifact: CORE_CONTENT_META.clone()
            },
            Payload{
                buffer: content.data.clone(),
                artifact: artifact
            }
        ];

        return Ok(rtn);
    }
}

impl ReadOnlyContentMeta for ReadOnlyContent
{
    fn get_artifact(&self) -> Result<Artifact, Box<dyn Error>> {
        Ok(Artifact::from(self.meta.get(&path!["artifact"] )?)?)
    }

    fn get_creation_timestamp(&self) -> Result<i64, Box<dyn Error>> {
        Ok(self.meta.get(&path!["creation_timestamp"] )?)
    }

    fn get_creation_cycle(&self) -> Result<i64, Box<dyn Error>> {
        Ok(self.meta.get(&path!["creation_cycle"] )?)
    }
}

trait ReadOnlyContentMeta
{
    fn get_artifact(&self)->Result<Artifact,Box<dyn Error>>;
    fn get_creation_timestamp(&self)->Result<i64,Box<dyn Error>>;
    fn get_creation_cycle(&self)->Result<i64,Box<dyn Error>>;
}

trait ContentMeta: ReadOnlyContentMeta
{
    fn set_artifact(&mut self,artifact:&Artifact)->Result<bool,Box<dyn Error>>;
    fn set_creation_timestamp(&mut self,value:i64)->Result<bool,Box<dyn Error>>;
    fn set_creation_cycle(&mut self,value: i64)->Result<bool,Box<dyn Error>>;
}

