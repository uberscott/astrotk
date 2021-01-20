use crate::artifact_config::{ArtifactFile, ArtifactFileYaml, Artifact, ArtifactCacher, ArtifactRepository};
use serde::{Serialize,Deserialize};


pub struct ActorConfig {
    pub source: ArtifactFile,
    pub wasm: ArtifactFile,
    pub name: String,
    pub content: ArtifactFile
}

impl ArtifactCacher for ActorConfig {
    fn cache(&self, repo: &mut ArtifactRepository) -> Result<(),Box<std::error::Error>>
    {
        repo.cache_file_as_string(&self.source)?;
        repo.cache_file_as_string(&self.content)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ActorConfigYaml
{
    name: String,
    content: ArtifactFileYaml,
    wasm: ArtifactFileYaml
}

impl ActorConfigYaml {

    pub fn from_yaml(string:&str) -> Result<Self,Box<std::error::Error>>
    {
       Ok(serde_yaml::from_str(string )?)
    }

    pub fn to_actor_config(&self, artifact_file: &ArtifactFile ) -> Result<ActorConfig,Box<std::error::Error>>
    {
        return Ok( ActorConfig{
            source: artifact_file.clone(),
            name: self.name.clone(),
            content: self.content.to_artifact_file(&artifact_file.artifact.clone())?,
            wasm: self.wasm.to_artifact_file(&artifact_file.artifact.clone())?
        } )
    }
}

