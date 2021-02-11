use no_proto::buffer::NP_Buffer;
use no_proto::NP_Factory;

use crate::artifact::Artifact;
use std::error::Error;
use no_proto::memory::NP_Memory_Owned;

pub trait BufferFactories
{
    fn create_buffer(&self, artifact_file: &Artifact) ->Result<NP_Buffer<NP_Memory_Owned>,Box<dyn Error>>;
    fn create_buffer_from(&self, artifact_file: &Artifact, array: Vec<u8> ) ->Result<NP_Buffer<NP_Memory_Owned>,Box<dyn Error>>;
    fn get_buffer_factory(&self, artifact_file: &Artifact) ->Option<&'static NP_Factory<'static>>;
}