

use semver::{Version, SemVerError};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io;
use astrotk_config::artifact_config::{ArtifactFile, Artifact, ArtifactRepository};
use std::sync::{Arc, Mutex};


pub struct Repository
{
    repo_path: String,
    cache : HashMap<ArtifactFile,String>
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
    fn fetch_artifact( &mut self, artifact: &Artifact ) -> Result<(),io::Error>
    {
        // at this time we don't do anything
        // later we will pull a zip file from a public repository and
        // extract the files to 'repo_path'
        return Ok(());
    }

    fn cache_file_as_string(&mut self, artifact_file: &ArtifactFile ) -> Result<(),Box<std::error::Error>>
    {
        if self.cache.contains_key(artifact_file )
        {
            return Ok(());
        }
        let string = String::from_utf8(self.load_file(artifact_file)? )?;
        self.cache.insert( artifact_file.clone(), string );
        return Ok(());
    }

    fn load_file( &self, artifact_file: &ArtifactFile ) -> Result<Vec<u8>,io::Error>
    {
        let mut path = String::new();
        path.push_str( self.repo_path.as_str() );
        if !self.repo_path.ends_with("/")
        {
            path.push_str( "/" );
        }
        path.push_str( artifact_file.artifact.group.as_str() );
        path.push_str( "/" );
        path.push_str( artifact_file.artifact.id.as_str() );
        path.push_str( "/" );
        path.push_str( artifact_file.artifact.version.to_string().as_str() );
        path.push_str( "/" );
        path.push_str( artifact_file.path.as_str() );

        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        return Ok(data);
    }

    fn get_cached_string(&self, artifact_file:&ArtifactFile ) -> Option<&String>
    {
        return  self.cache.get( artifact_file );
    }

}