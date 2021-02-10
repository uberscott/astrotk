use std::borrow::BorrowMut;
use std::cell::Cell;
use std::collections::HashMap;
use std::error::Error;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

use no_proto::NP_Factory;
use serde::{Deserialize, Serialize};

use crate::artifact::{Artifact, ArtifactBundle, ArtifactCache, ArtifactCacher, ArtifactRepository, ArtifactYaml};

pub struct Configs
{
    pub artifact_cache: Arc<dyn ArtifactCache+Sync+Send>,
    pub buffer_factory_keeper: Keeper<NP_Factory<'static>>,
    pub sim_config_keeper: Keeper<SimConfig>,
    pub mechtron_config_keeper: Keeper<MechtronConfig>,
}

impl Configs{

    pub fn new(artifact_source:Arc<dyn ArtifactCache+Sync+Send>)->Self
    {
        Configs{
            artifact_cache: artifact_source.clone(),
            buffer_factory_keeper: Keeper::new(artifact_source.clone() , Box::new(NP_Buffer_Factory_Parser )),
            sim_config_keeper: Keeper::new(artifact_source.clone(), Box::new( SimConfigParser )),
            mechtron_config_keeper: Keeper::new(artifact_source.clone(), Box::new( MechtronConfigParser ))
        }
    }
}



pub struct Keeper<V>
{
    config_cache: RwLock<HashMap<Artifact,Arc<V>>>,
    repo: Arc<dyn ArtifactCache+Send+Sync>,
    parser: Box<dyn Parser<V> + Send+Sync>
}

impl <V> Keeper<V>
{
    pub fn new(repo: Arc<dyn ArtifactCache + Send + Sync>, parser: Box<dyn Parser<V> + Send + Sync>) -> Self
    {
        Keeper {
            config_cache: RwLock::new(HashMap::new()),
            parser: parser,
            repo: repo
        }
    }

    pub fn cache(&mut self, artifact: &Artifact ) ->Result<(),Box<dyn Error + '_>>
    {
        let mut cache = self.config_cache.write()?;

        if cache.contains_key(artifact)
        {
            return Ok(());
        }

        self.repo.cache(&artifact);


        let str = self.repo.get(&artifact)?;

        let value = self.parser.parse(&artifact, str.as_ref())?;
        cache.insert( artifact.clone(), Arc::new(value) );
        Ok(())
    }

    pub fn get( &self, artifact: &Artifact ) -> Result<Arc<V>,Box<dyn Error + '_>>
    {
        let cache = self.config_cache.read()?;
        match cache.get(&artifact)
        {
            None => Err(format!("could not find config for artifact: {}",artifact.to()).into()),
            Some(value) => Ok(value.clone())
        }
    }
}

pub trait Parser<V>
{
    fn parse( &self, artifact: &Artifact, str: &str )->Result<V,Box<dyn Error>>;
}


struct NP_Buffer_Factory_Parser;

impl <'fact> Parser<NP_Factory<'fact>> for NP_Buffer_Factory_Parser
{
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<NP_Factory<'fact>, Box<dyn Error>> {
        let result = NP_Factory::new(str);
        match result {
            Ok(rtn) => Ok(rtn),
            Err(e) => Err(format!("could not parse np_factory from artifact: {} error is: {}",artifact.to(),e.message).into())
        }
    }
}

struct SimConfigParser;

impl Parser<SimConfig> for SimConfigParser
{
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<SimConfig, Box<dyn Error>> {
        let sim_config_yaml = SimConfigYaml::from(str)?;
        let sim_config = sim_config_yaml.to_config(artifact)?;
        Ok(sim_config)
    }
}

struct MechtronConfigParser;

impl Parser<MechtronConfig> for MechtronConfigParser
{
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<MechtronConfig, Box<dyn Error>> {
        let mechtron_config_yaml = MechtronConfigYaml::from_yaml(str)?;
        let mechtron_config = mechtron_config_yaml.to_config(artifact)?;
        Ok(mechtron_config)
    }
}


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
    fn cache(&self, _: &mut Arc<Cell<Configs>>) -> Result<(), Box<dyn Error>> {
        {
/*            let configs = (*configs).get_mut();
            configs.buffer_factory_keeper.cache(&self.content);
            configs.buffer_factory_keeper.cache(&self.create_message);

 */
            Ok(())
        }
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

    pub fn to_config(&self, artifact_file: &Artifact) -> Result<MechtronConfig,Box<dyn Error>>
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


pub struct SimConfig{
    source: Artifact,
    name: String,
    main: Artifact,
    create_message: Artifact
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct SimConfigYaml
{
    name: String,
    main: ArtifactYaml,
    create_message: ArtifactYaml
}

impl SimConfigYaml
{
    pub fn from(string:&str) -> Result<Self,Box<dyn Error>>
    {
        Ok(serde_yaml::from_str(string )?)
    }

    pub fn to_config(&self, artifact: &Artifact ) -> Result<SimConfig,Box<dyn Error>>
    {
        let default_artifact = &artifact.bundle.clone();
        Ok( SimConfig{
            source: artifact.clone(),
            name: self.name.clone(),
            main: self.main.to_artifact_file(default_artifact)?,
            create_message: self.create_message.to_artifact_file(default_artifact)?
        } )
    }
}

impl ArtifactCacher for SimConfig {
    fn cache(&self, configs: &mut Arc<Cell<Configs>>) -> Result<(), Box<dyn Error>> {
/*        let configs = (*configs).get_mut();
        configs.buffer_factory_keeper.cache(&self.create_message);
        configs.mechtron_config_keeper.cache(&self.main);

 */
        Ok(())
    }
}

