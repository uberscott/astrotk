use crate::artifact_config::ArtifactFile;
use no_proto::buffer::NP_Buffer;
use no_proto::NP_Factory;

pub trait BufferFactories
{
    fn create_buffer( &self, artifact_file: &ArtifactFile )->Result<NP_Buffer,Box<std::error::Error>>;
    fn create_buffer_from( &self, artifact_file: &ArtifactFile, array: Vec<u8> )->Result<NP_Buffer,Box<std::error::Error>>;
    fn get_buffer_factory( &self, artifact_file: &ArtifactFile )->Option<&NP_Factory>;
}