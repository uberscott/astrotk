use crate::id::ContentKey;
use no_proto::buffer::{NP_Buffer, NP_Finished_Buffer};
use std::sync::Arc;
use no_proto::memory::{NP_Memory_Owned, NP_Memory_Ref};
use crate::artifact::Artifact;
use no_proto::NP_Factory;
use crate::buffers::BufferFactories;
use crate::configs::{Keeper, Configs};
use no_proto::error::NP_Error;
use std::error::Error;
use crate::message::Payload;


#[derive(Clone)]
pub struct Content
{
    pub artifact: Artifact,
    pub meta: NP_Buffer<NP_Memory_Owned>,
    pub data: NP_Buffer<NP_Memory_Owned>
}

impl Content
{
    pub fn new(  configs: &Configs, artifact: Artifact )->Self
    {
        let data = configs.buffer_factory_keeper.get(&artifact).unwrap().new_buffer(Option::None);
        let meta=configs.core_buffer_factory("schema/content/meta")?.new_buffer(Option::None);
        Content{
            artifact: artifact,
            meta:meta,
            data:data
        }
    }

    pub fn from( artifact:Artifact, meta: NP_Buffer<NP_Memory_Owned>, data: NP_Buffer<NP_Memory_Owned> ) -> Self
    {
        Content {
            artifact: artifact,
            meta: meta,
            data: data
        }
    }

    pub fn read_only( &self ) -> Result<ReadOnlyContent,Box<dyn Error>>
    {
        Ok(ReadOnlyContent{
            artifact: self.artifact.clone(),
            meta: self.meta.finish(),
            data: self.data.finish()
        })
    }

    pub fn compact( &mut self ) -> Result<(),Box<dyn Error>>
    {
        match self.compact_inner()
        {
            Ok(()) => Ok(()),
            Err(e) => Err("could not compact content due to an error".into())
        }
    }

    fn compact_inner( &mut self ) -> Result<(),NP_Error>
    {
        self.meta.compact(Option::None)?;
        self.data.compact(Option::None)?;
        Ok(())
    }

    // terrible that I have to clone the buffers here... i wish there was a way to do this in read only buffers
    pub fn payloads(&self, configs: &Configs )->Vec<Payload>
    {
        let rtn : Vec<Payload> = vec![
            Payload{
                buffer: Arc::new(self.meta.clone() ),
                artifact: configs.core_artifact("schema/content/meta")?
            },
            Payload{
                buffer: Arc::new(self.data.clone() ),
                artifact: self.artifact.clone()
            }
        ];

        return rtn;
    }
}


pub struct ReadOnlyContent
{
    pub artifact: Artifact,
    pub meta: NP_Finished_Buffer<NP_Memory_Owned>,
    pub data: NP_Finished_Buffer<NP_Memory_Owned>,
}

impl ReadOnlyContent
{
    pub fn copy( &self )->Result<Content,Box<dyn Error>>
    {
        Ok(Content {
            meta: configs.core_buffer_factory("schema/content/meta")?.open_buffer(self.meta.bytes()),
            data: configs.buffer_factory_keeper.get(&self.artifact)?.open_buffer(self.data.bytes()),
            artifact: self.artifact.clone()
        })
    }
}
