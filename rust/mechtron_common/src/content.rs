use crate::id::ContentKey;
use no_proto::buffer::NP_Buffer;
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

    pub fn read_only( &self, configs: &Configs ) -> Result<ReadOnlyContent<'_>,Box<dyn Error>>
    {
        let ro_meta =configs.core_buffer_factory("schema/content/meta")?.open_buffer_ref(self.meta.read_bytes());
        let data_factory = configs.buffer_factory_keeper.get(&self.artifact)?;
        let ro_data = data_factory.open_buffer_ref(self.data.read_bytes() );
        Ok(ReadOnlyContent{
            artifact: self.artifact.clone(),
            meta: Arc::new(ro_meta),
            data: Arc::new(ro_data)
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

pub struct ReadOnlyContent<'buffers>
{
    pub artifact: Artifact,
    pub meta: Arc<NP_Buffer<NP_Memory_Ref<'buffers>>>,
    pub data: Arc<NP_Buffer<NP_Memory_Ref<'buffers>>>
}

impl <'buffers> ReadOnlyContent<'buffers>
{

}
