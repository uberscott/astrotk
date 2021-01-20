use semver::Version;
use serde::*;
use std::io;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash,Debug,Clone)]
pub struct Artifact
{
    pub group: String,
    pub id: String,
    pub version: Version
}

impl Artifact {
    pub fn parse( string: &str ) -> Result<Self,Box<std::error::Error>> {
        let mut parts = string.split(":" );

        return Ok(Artifact {
            group: parts.next().unwrap().to_string(),
            id: parts.next().unwrap().to_string(),
            version: Version::parse( parts.next().unwrap() )?
        });
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash,Debug,Clone)]
pub struct ArtifactFile
{
    pub artifact: Artifact,
    pub path: String
}

impl ArtifactFile
{
    pub fn from(string: &str ) -> Result<Self,Box<std::error::Error>> {
        let mut parts = string.split(":");
        return Ok(ArtifactFile{artifact: Artifact {
            group: parts.next().unwrap().to_string(),
            id: parts.next().unwrap().to_string(),
            version: Version::parse( parts.next().unwrap() )?
        },
            path: parts.next().unwrap().to_string()
        });
    }

    pub fn to(&self) -> String
    {
        let mut rtn = String::new();
        rtn.push_str(self.artifact.group.as_str() );
        rtn.push_str(":" );
        rtn.push_str(self.artifact.id.as_str() );
        rtn.push_str(":" );
        rtn.push_str(self.artifact.version.to_string().as_str() );
        rtn.push_str(":" );
        rtn.push_str(self.path.as_str() );
        let rtn = rtn;
        return rtn;
    }
}



#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ArtifactFileYaml{
    pub artifact: Option<String>,
    pub path: String
}

impl ArtifactFileYaml {
    pub fn to_artifact_file(&self, default_artifact: &Artifact ) -> Result<ArtifactFile,Box<std::error::Error>>
    {
        let artifact = self.artifact.clone();
        return Ok( ArtifactFile{
            artifact: match artifact {
                None => default_artifact.clone(),
                Some(artifact) => Artifact::parse(artifact.as_str() )?
            },
            path: self.path.clone()
        });
    }
}

pub trait ArtifactRepository
{
    fn fetch_artifact( &mut self, artifact: &Artifact ) -> Result<(),io::Error>;

    fn cache_file_as_string(&mut self, artifact_file: &ArtifactFile ) -> Result<(),Box<std::error::Error>>;

    fn load_file( &self, artifact_file: &ArtifactFile ) -> Result<Vec<u8>,io::Error>;

    fn get_cached_string(&self, artifact_file:&ArtifactFile ) -> Option<&String>;
}

pub trait ArtifactCacher
{
    fn cache(&self, repo:&mut ArtifactRepository) -> Result<(),Box<std::error::Error>>;
}
