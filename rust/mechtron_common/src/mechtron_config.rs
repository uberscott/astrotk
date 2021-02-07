use serde::{Deserialize, Serialize};

use crate::artifact::{ArtifactCacher, Artifact, ArtifactYaml, ArtifactRepository};

use std::error::Error;

pub struct MechtronConfig {
    pub source: Artifact,
    pub wasm: Artifact,
    pub name: String,
    pub content: Artifact,
    pub create_message: Artifact,
}

impl MechtronConfig {
    pub fn message_artifact_files(&self) ->Vec<Artifact>
    {
        let mut rtn: Vec<Artifact> =Vec::new();
        rtn.push( self.content.clone() );
        rtn.push( self.create_message.clone() );
        return rtn;
    }
}

impl ArtifactCacher for MechtronConfig {

    fn cache(&self, repo: &mut dyn ArtifactRepository) -> Result<(),Box<dyn Error>>
    {
        repo.cache_file_as_string(&self.source)?;
        repo.cache_file_as_string(&self.content)?;
        repo.cache_file_as_string(&self.create_message)?;
        Ok(())
    }

}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MechtronConfigYaml
{
    name: String,
    content: ArtifactYaml,
    wasm: ArtifactYaml,
    create_message: ArtifactYaml
}

impl MechtronConfigYaml {

    pub fn from_yaml(string:&str) -> Result<Self,Box<dyn Error>>
    {
       Ok(serde_yaml::from_str(string )?)
    }

    pub fn to_actor_config(&self, artifact_file: &Artifact) -> Result<MechtronConfig,Box<dyn Error>>
    {
        let default_artifact = &artifact_file.bundle.clone();
        return Ok( MechtronConfig {
            source: artifact_file.clone(),
            name: self.name.clone(),
            content: self.content.to_artifact_file(default_artifact)?,
            wasm: self.wasm.to_artifact_file(default_artifact)?,
            create_message: self.create_message.to_artifact_file(default_artifact)?
        } )
    }
}

