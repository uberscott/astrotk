use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::error::Error;

use semver::Version;
use serde::{Deserialize, Serialize};

pub struct Repository
{
    repo_path: String,
    cache : HashMap<Artifact,String>
}


impl Repository
{
    pub fn new(repo_path: String) -> Self
    {
        return Repository {
            repo_path: repo_path,
            cache: HashMap::new()
        };
    }
}

impl ArtifactRepository for Repository
{
    fn fetch_artifact_bundle(&mut self, bundle: &ArtifactBundle) -> Result<(),io::Error>
    {
        // at this time we don't do anything
        // later we will pull a zip file from a public repository and
        // extract the files to 'repo_path'
        return Ok(());
    }

    fn cache_file_as_string(&mut self, artifact_file: &Artifact) -> Result<(),Box<dyn Error>>
    {
        if self.cache.contains_key(artifact_file )
        {
            return Ok(());
        }
        let string = String::from_utf8(self.load_file(artifact_file)? )?;
        self.cache.insert( artifact_file.clone(), string );
        return Ok(());
    }

    fn load_file(&self, artifact_file: &Artifact) -> Result<Vec<u8>,io::Error>
    {
        let mut path = String::new();
        path.push_str( self.repo_path.as_str() );
        if !self.repo_path.ends_with("/")
        {
            path.push_str( "/" );
        }
        path.push_str( artifact_file.bundle.group.as_str() );
        path.push_str( "/" );
        path.push_str( artifact_file.bundle.id.as_str() );
        path.push_str( "/" );
        path.push_str( artifact_file.bundle.version.to_string().as_str() );
        path.push_str( "/" );
        path.push_str( artifact_file.path.as_str() );

        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        return Ok(data);
    }

    fn get_cached_string(&self, artifact_file:&Artifact) -> Option<&String>
    {
        return  self.cache.get( artifact_file );
    }

}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash,Debug,Clone)]
pub struct ArtifactBundle
{
    pub group: String,
    pub id: String,
    pub version: Version
}

impl ArtifactBundle {
    pub fn parse( string: &str ) -> Result<Self,Box<dyn Error>> {
        let mut parts = string.split(":" );

        return Ok(ArtifactBundle {
            group: parts.next().unwrap().to_string(),
            id: parts.next().unwrap().to_string(),
            version: Version::parse( parts.next().unwrap() )?
        });
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash,Debug,Clone)]
pub struct Artifact
{
    pub bundle: ArtifactBundle,
    pub path: String
}

impl Artifact
{
    pub fn from(string: &str ) -> Result<Self,Box<dyn Error>> {
        let mut parts = string.split(":");
        return Ok(Artifact {
            bundle: ArtifactBundle {
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
        rtn.push_str(self.bundle.group.as_str() );
        rtn.push_str(":" );
        rtn.push_str(self.bundle.id.as_str() );
        rtn.push_str(":" );
        rtn.push_str(self.bundle.version.to_string().as_str() );
        rtn.push_str(":" );
        rtn.push_str(self.path.as_str() );
        let rtn = rtn;
        return rtn;
    }
}



#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ArtifactYaml {
    pub bundle: Option<String>,
    pub path: String
}

impl ArtifactYaml {
    pub fn to_artifact_file(&self, default_artifact: &ArtifactBundle) -> Result<Artifact,Box<dyn Error>>
    {
        let artifact = self.bundle.clone();
        return Ok( Artifact {
            bundle: match artifact {
                None => default_artifact.clone(),
                Some(artifact) => ArtifactBundle::parse(artifact.as_str() )?
            },
            path: self.path.clone()
        });
    }
}

pub trait ArtifactRepository
{
    fn fetch_artifact_bundle(&mut self, artifact: &ArtifactBundle) -> Result<(),io::Error>;

    fn cache_file_as_string(&mut self, artifact_file: &Artifact) -> Result<(),Box<dyn Error>>;

    fn load_file(&self, artifact_file: &Artifact) -> Result<Vec<u8>,io::Error>;

    fn get_cached_string(&self, artifact_file:&Artifact) -> Option<&String>;
}

pub trait ArtifactCacher
{
    fn cache(&self, repo:&mut dyn ArtifactRepository) -> Result<(),Box<dyn Error>>;
}
