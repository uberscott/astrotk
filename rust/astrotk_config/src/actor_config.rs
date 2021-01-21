use crate::artifact_config::{ArtifactFile, ArtifactFileYaml, Artifact, ArtifactCacher, ArtifactRepository};
use serde::{Serialize,Deserialize};


pub struct ActorConfig {
    pub source: ArtifactFile,
    pub wasm: ArtifactFile,
    pub name: String,
    pub content: ArtifactFile,
    pub create_message: ArtifactFile,
}

impl ActorConfig{
    pub fn message_artifact_files(&self) ->Vec<ArtifactFile>
    {
        let mut rtn: Vec<ArtifactFile> =Vec::new();
        rtn.push( self.content.clone() );
        rtn.push( self.create_message.clone() );
        return rtn;
    }
}

impl ArtifactCacher for ActorConfig {

    fn cache(&self, repo: &mut ArtifactRepository) -> Result<(),Box<std::error::Error>>
    {
        repo.cache_file_as_string(&self.source)?;
        repo.cache_file_as_string(&self.content)?;
        repo.cache_file_as_string(&self.create_message)?;
        Ok(())
    }


}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ActorConfigYaml
{
    name: String,
    content: ArtifactFileYaml,
    wasm: ArtifactFileYaml,
    create_message: ArtifactFileYaml,
}

impl ActorConfigYaml {

    pub fn from_yaml(string:&str) -> Result<Self,Box<std::error::Error>>
    {
       Ok(serde_yaml::from_str(string )?)
    }

    pub fn to_actor_config(&self, artifact_file: &ArtifactFile ) -> Result<ActorConfig,Box<std::error::Error>>
    {
        let default_artifact = &artifact_file.artifact.clone();
        return Ok( ActorConfig{
            source: artifact_file.clone(),
            name: self.name.clone(),
            content: self.content.to_artifact_file(default_artifact)?,
            wasm: self.wasm.to_artifact_file(default_artifact)?,
            create_message: self.create_message.to_artifact_file(default_artifact)?
        } )
    }
}

