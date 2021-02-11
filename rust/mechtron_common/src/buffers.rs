use no_proto::buffer::NP_Buffer;
use no_proto::NP_Factory;

use crate::artifact::Artifact;
use std::error::Error;
use no_proto::memory::NP_Memory_Owned;

pub trait BufferFactories
{
    fn create_buffer(&self, artifact: &Artifact) ->Result<NP_Buffer<NP_Memory_Owned>,Box<dyn Error>>;
    fn create_buffer_from_array(&self, artifact: &Artifact, array: Vec<u8> ) ->Result<NP_Buffer<NP_Memory_Owned>,Box<dyn Error>>;
    fn create_buffer_from_buffer(&self, artifact: &Artifact, buffer: NP_Buffer<NP_Memory_Owned> ) ->Result<NP_Buffer<NP_Memory_Owned>,Box<dyn Error>>;
    fn get_buffer_factory(&self, artifact: &Artifact) ->Option<&'static NP_Factory<'static>>;
}