use mechtron_core::artifact::{Artifact, ArtifactBundle, ArtifactCache, ArtifactRepository};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::ops::Deref;
use std::sync::{Arc, Mutex, RwLock};
use std::env;
use mechtron_core::error::Error;

pub struct FileSystemArtifactRepository {
    repo_path: String,
    cache: RwLock<HashMap<Artifact, Arc<String>>>,
    fetches: RwLock<HashSet<ArtifactBundle>>,
}

impl FileSystemArtifactRepository {
    pub fn new(repo_path: &str ) -> Self {
        return FileSystemArtifactRepository {
            repo_path: repo_path.to_string(),
            cache: RwLock::new(HashMap::new()),
            fetches: RwLock::new(HashSet::new()),
        };
    }
}

impl ArtifactRepository for FileSystemArtifactRepository {
    fn fetch(&self, bundle: &ArtifactBundle) -> Result<(), Error> {
        {
            let lock = self.fetches.read()?;
            if lock.contains(bundle) {
                return Ok(());
            }
        }

        let mut lock = self.fetches.write()?;
        lock.insert(bundle.clone());

        // at this time we don't do anything
        // later we will pull a zip file from a public repository and
        // extract the files to 'repo_path'
        return Ok(());
    }
}

impl ArtifactCache for FileSystemArtifactRepository {
    fn cache(&self, artifact: &Artifact) -> Result<(), Error > {


        self.fetch( &artifact.bundle )?;

        let mut cache = self.cache.write()?;
        if cache.contains_key(artifact) {
            return Ok(());
        }
        //        let mut cache = cell.borrow_mut();
        let string = String::from_utf8(self.load(artifact)?)?;
        cache.insert(artifact.clone(), Arc::new(string));
        return Ok(());
    }

    fn load(&self, artifact: &Artifact) -> Result<Vec<u8>, Error > {
        {
            let lock = self.fetches.read()?;
            if !lock.contains(&artifact.bundle) {
                return Err(format!(
                    "fetch must be called on bundle: {} before artifact can be loaded: {}",
                    artifact.bundle.to(),
                    artifact.to()
                )
                    .into());
            }
        }

        let mut path = String::new();
        path.push_str(self.repo_path.as_str());
        if !self.repo_path.ends_with("/") {
            path.push_str("/");
        }
        path.push_str(artifact.bundle.group.as_str());
        path.push_str("/");
        path.push_str(artifact.bundle.id.as_str());
        path.push_str("/");
        path.push_str(artifact.bundle.version.to_string().as_str());
        path.push_str("/");
        path.push_str(artifact.path.as_str());

        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        return Ok(data);
    }

    fn get(&self, artifact: &Artifact) -> Result<Arc<String>, Error > {
        let cache = self.cache.read()?;
        let option = cache.get(artifact);

        match option {
            None => Err(format!("artifact is not cached: {}", artifact.to()).into()),
            Some(rtn) => Ok(rtn.clone()),
        }
    }
}
