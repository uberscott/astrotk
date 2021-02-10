use std::borrow::Borrow;
use std::cell::{Cell, Ref, RefCell};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::configs::Configs;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub struct ArtifactBundle
{
    pub group: String,
    pub id: String,
    pub version: Version
}

impl ArtifactBundle {
    pub fn parse(string: &str) -> Result<Self, Box<dyn Error>> {
        let mut parts = string.split(":");

        return Ok(ArtifactBundle {
            group: parts.next().unwrap().to_string(),
            id: parts.next().unwrap().to_string(),
            version: Version::parse(parts.next().unwrap())?,
        });
    }

    pub fn to(&self) -> String
    {
        let mut rtn = String::new();
        rtn.push_str(self.group.as_str());
        rtn.push_str(":");
        rtn.push_str(self.id.as_str());
        rtn.push_str(":");
        rtn.push_str(self.version.to_string().as_str());
        let rtn = rtn;
        return rtn;
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
    fn fetch(&self, bundle: &ArtifactBundle) -> Result<(), Box<dyn Error + '_>>;
}

pub struct ArtifactCacheMutex{
    cache: Arc<Mutex<Box<dyn ArtifactCache>>>
}

impl ArtifactCacheMutex{
    pub fn new( cache: Arc<Mutex<Box<dyn ArtifactCache>>> )->Self
    {
        ArtifactCacheMutex{
            cache: cache
        }
    }
}

impl ArtifactCache for ArtifactCacheMutex{


    fn cache(&self, artifact: &Artifact) -> Result<(), Box<dyn Error+'_>> {
        let cache = self.cache.lock()?;
        let result = cache.cache(artifact);
        match result
        {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("issue when trying to cache artifact: {:?} received message: {}",artifact,e.to_string()).into())
        }

    }

    fn load(&self, artifact: &Artifact) -> Result<Vec<u8>, Box<dyn Error+'_>> {
        let cache = self.cache.lock()?;
        let rtn = cache.load(artifact);
        match rtn
        {
            Ok(r) => Ok(r),
            Err(e) => Err(format!("issue when trying to load artifact: {:?} received message: {}",artifact,e.to_string()).into())
        }
    }

    fn get(&self, artifact: &Artifact) -> Result<Arc<String>, Box<dyn Error+'_>> {
        let cache = self.cache.lock()?;
        let rtn = cache.get(artifact);
        match rtn
        {
            Ok(r) => Ok(r),
            Err(e) => Err(format!("issue when trying to get artifact: {:?} received message: {}",artifact,e.to_string()).into())
        }
    }
}

pub trait ArtifactCache: Send + Sync
{
    fn cache(&self, artifact: &Artifact) -> Result<(), Box<dyn Error + '_>>;

    fn load(&self, artifact: &Artifact) -> Result<Vec<u8>, Box<dyn Error + '_>>;

    fn get(&self, artifact: &Artifact) -> Result<Arc<String>, Box<dyn Error + '_>>;
}

pub trait ArtifactCacher
{
    fn cache(&self, configs: &mut Arc<Cell<Configs>>) -> Result<(), Box<dyn Error + '_>>;
}
