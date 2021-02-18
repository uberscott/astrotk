
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::configs::Configs;
use std::cell::Cell;
use std::sync::Arc;
use crate::error::Error;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub struct ArtifactBundle {
    pub group: String,
    pub id: String,
    pub version: Version,
}

impl ArtifactBundle {
    pub fn parse(string: &str) -> Result<Self, Error> {
        let mut parts = string.split(":");

        return Ok(ArtifactBundle {
            group: parts.next().unwrap().to_string(),
            id: parts.next().unwrap().to_string(),
            version: Version::parse(parts.next().unwrap())?,
        });
    }

    pub fn to(&self) -> String {
        let mut rtn = String::new();
        rtn.push_str(self.group.as_str());
        rtn.push_str(":");
        rtn.push_str(self.id.as_str());
        rtn.push_str(":");
        rtn.push_str(self.version.to_string().as_str());
        let rtn = rtn;
        return rtn;
    }

    pub fn path(&self, string: &str) -> Artifact {
        Artifact {
            bundle: self.clone(),
            path: string.to_string(),
            kind: Option::None
        }
    }

    pub fn path_and_kind(&self, path : &str, kind: &str ) -> Artifact {
        Artifact {
            bundle: self.clone(),
            path: path.to_string(),
            kind: Option::Some(kind.to_string())
        }
    }

}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub struct Artifact {
    pub bundle: ArtifactBundle,
    pub path: String,
    pub kind: Option<String>

}

impl Artifact {
    pub fn from(string: &str) -> Result<Self, Error> {
        let mut parts = string.split(":");

        return Ok(Artifact {
            bundle: ArtifactBundle {
                group: parts.next().unwrap().to_string(),
                id: parts.next().unwrap().to_string(),
                version: Version::parse(parts.next().unwrap())?,
            },
            path: parts.next().unwrap().to_string(),
            kind: match parts.next(){
                None => Option::None,
                Some(s) => Option::Some(s.to_string())
            }
        });
    }

    pub fn to(&self) -> String {
        let mut rtn = String::new();
        rtn.push_str(self.bundle.group.as_str());
        rtn.push_str(":");
        rtn.push_str(self.bundle.id.as_str());
        rtn.push_str(":");
        rtn.push_str(self.bundle.version.to_string().as_str());
        rtn.push_str(":");
        rtn.push_str(self.path.as_str());

        if self.kind.is_some()
        {
            rtn.push_str(":");
            rtn.push_str(self.kind.as_ref().unwrap().as_str() );
        }

        let rtn = rtn;
        return rtn;
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ArtifactYaml {
    pub bundle: Option<String>,
    pub path: String,
    pub kind: Option<String>,
}

impl ArtifactYaml {
    pub fn to_artifact(&self, default_bundle: &ArtifactBundle, kind: Option<&str> ) -> Result<Artifact, Error> {
        let artifact = self.bundle.clone();
        return Ok(Artifact {
            bundle: match artifact {
                None => default_bundle.clone(),
                Some(artifact) => ArtifactBundle::parse(artifact.as_str())?,
            },
            path: self.path.clone(),
            kind: match &self.kind {
                Some(kind)=>Option::Some(kind.to_string()),
                None=>match kind {
                    None => None,
                    Some(str) => Option::Some(str.to_string())
                }
            }
        });
    }
}

pub trait ArtifactRepository {
    fn fetch(&self, bundle: &ArtifactBundle) -> Result<(), Error >;
}

pub trait ArtifactCache: Send + Sync {
    fn cache(&self, artifact: &Artifact) -> Result<(), Error>;

    fn load(&self, artifact: &Artifact) -> Result<Vec<u8>, Error>;

    fn get(&self, artifact: &Artifact) -> Result<Arc<String>, Error >;
}


