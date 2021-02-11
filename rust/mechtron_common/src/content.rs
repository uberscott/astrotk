use crate::id::ContentKey;
use no_proto::buffer::NP_Buffer;
use std::sync::Arc;
use no_proto::memory::{NP_Memory_Owned, NP_Memory_Ref};
use crate::artifact::Artifact;
use no_proto::NP_Factory;
use crate::buffers::BufferFactories;
use crate::configs::Keeper;
use no_proto::error::NP_Error;
use std::error::Error;

static CONTENT_META_SCHEMA: &'static str = r#"{
    "type": "table",
    "columns": [
               ["creation_timestamp", {"type":"i64"}],
               ["creation_cycle",     {"type":"i64"}]
               ]
}"#;

lazy_static! {
  pub static ref CONTENT_META_FACTORY: NP_Factory<'static> = NP_Factory::new(CONTENT_META_SCHEMA).unwrap();
}


#[derive(Clone)]
pub struct Content
{
    pub artifact: Artifact,
    pub meta: NP_Buffer<NP_Memory_Owned>,
    pub data: NP_Buffer<NP_Memory_Owned>
}

impl Content
{
    pub fn new(  buffer_factories: &Keeper<NP_Factory<'static>>, artifact: Artifact )->Self
    {
        let data = buffer_factories.get(&artifact).unwrap().new_buffer(Option::None);
        let meta= CONTENT_META_FACTORY.new_buffer(Option::None);
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

    pub fn read_only( &self, buffer_factories: &Keeper<NP_Factory>) -> Result<ReadOnlyContent<'_>,Box<dyn Error>>
    {
        let ro_meta = CONTENT_META_FACTORY.open_buffer_ref(self.meta.read_bytes());
        let data_factory = buffer_factories.get(&self.artifact)?;
        let ro_data = data_factory.open_buffer_ref(self.data.read_bytes() );
        Ok(ReadOnlyContent{
            artifact: self.artifact.clone(),
            meta: ro_meta,
            data: ro_data
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

}

struct ReadOnlyContent<'buffers>
{
    pub artifact: Artifact,
    pub meta: NP_Buffer<NP_Memory_Ref<'buffers>>,
    pub data: NP_Buffer<NP_Memory_Ref<'buffers>>
}

