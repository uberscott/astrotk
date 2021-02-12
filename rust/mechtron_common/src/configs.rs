use std::borrow::BorrowMut;
use std::cell::Cell;
use std::collections::HashMap;
use std::error::Error;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

use no_proto::NP_Factory;
use serde::{Deserialize, Serialize};

use crate::artifact::{Artifact, ArtifactBundle, ArtifactCache, ArtifactCacher, ArtifactRepository, ArtifactYaml};


pub static CORE_BUNDLE: &'static str = "mechtron.io:core:0.0.1";
static CORE_BUNDLE_FMT: &'static str = "mechtron.io:core:0.0.1:{}";

pub struct Configs
{
    core_artifacts: HashMap<String,Artifact>,
    pub artifact_cache: Arc<dyn ArtifactCache+Sync+Send>,
    pub buffer_factory_keeper: Keeper<NP_Factory<'static>>,
    pub sim_config_keeper: Keeper<SimConfig>,
    pub tron_config_keeper: Keeper<TronConfig>,
    pub mechtron_config_keeper: Keeper<MechtronConfig>,
}

impl Configs{

    pub fn new(artifact_source:Arc<dyn ArtifactCache+Sync+Send>)->Self
    {
        let mut configs = Configs{
            core_artifacts: HashMap::new(),
            artifact_cache: artifact_source.clone(),
            buffer_factory_keeper: Keeper::new(artifact_source.clone() , Box::new(NP_Buffer_Factory_Parser )),
            sim_config_keeper: Keeper::new(artifact_source.clone(), Box::new( SimConfigParser )),
            tron_config_keeper: Keeper::new(artifact_source.clone(), Box::new( TronConfigParser )),
            mechtron_config_keeper: Keeper::new(artifact_source.clone(), Box::new( MechtronConfigParser ))
        };

        let version = "1.0.0";
        configs.core_artifacts.insert("tron/sim".to_string(), Artifact::from(format!("mechtron.io:core:{}:{}", version,"tron/sim.yaml").as_str())?);
        configs.core_artifacts.insert("tron/neutron".to_string(), Artifact::from(format!( "mechtron.io:core:{}:{}", version,"tron/neutron.yaml").as_str())?);
        configs.core_artifacts.insert("schema/empty".to_string(), Artifact::from(format!( "mechtron.io:core:{}:{}", version,"schema/empty.json").as_str())?);
        configs.core_artifacts.insert("schema/content/meta".to_string(), Artifact::from(format!( "mechtron.io:core:{}:{}", version,"schema/tron/content-meta.json").as_str())?);
        configs.core_artifacts.insert("schema/create/meta".to_string(), Artifact::from(format!( "mechtron.io:core:{}:{}", version,"schema/tron/create-meta.json").as_str())?);

        return configs;
    }

    pub fn core_artifact( &self, id: &str ) -> Result<Artifact,Box<dyn Error>>
    {
        match self.core_artifacts.get(id) {
            None => Err(format!("could not find core artifact {}",id).into()),
            Some(artifact) =>Ok(artifact.clone())
        }
    }

    pub fn core_tron_config(&self, id: &str ) -> Result<Arc<TronConfig>,Box<dyn Error>>
    {
        Ok(self.tron_config_keeper.get(&self.core_artifact(id)?)?.clone())
    }

    pub fn core_buffer_factory(&self, id: &str ) -> Result<Arc<NP_Factory<'static>>,Box<dyn Error>>
    {
        Ok(self.buffer_factory_keeper.get(&self.core_artifact(id)?)?.clone())
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
            Err(e) => Err(format!("could not parse np_factory from artifact: {}",artifact.to()).into())
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

struct TronConfigParser;

impl Parser<TronConfig> for TronConfigParser
{
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<TronConfig, Box<dyn Error>> {
        let tron_config_yaml = TronConfigYaml::from_yaml(str)?;
        let tron_config = tron_config_yaml.to_config(artifact)?;
        Ok(tron_config)
    }
}


#[derive(Clone)]
pub struct MechtronConfig {
    pub source: Artifact,
    pub wasm: Artifact,
    pub tron: TronConfigRef
}

#[derive(Clone)]
pub struct TronConfigRef
{
    pub artifact: Artifact
}

#[derive(Clone)]
pub struct TronConfig{
    pub kind: String,
    pub name: String,
    pub nucleus_lookup_name: Option<String>,
    pub source: Artifact,
    pub content: Option<ContentConfig>,
    pub messages: Option<MessagesConfig>
}

#[derive(Clone)]
pub struct MessagesConfig
{
    pub create: Option<CreateMessageConfig>
}

#[derive(Clone)]
pub struct ContentConfig
{
    pub artifact: Artifact
}

#[derive(Clone)]
pub struct CreateMessageConfig
{
    pub artifact: Artifact
}

#[derive(Clone)]
pub struct InboundMessageConfig
{
    pub name: String,
    pub phase: Option<String>,
    pub artifact: Vec<Artifact>
}

#[derive(Clone)]
pub struct OutboundMessageConfig
{
   pub name: String,
   pub artifact: Artifact
}

impl ArtifactCacher for TronConfig{
    fn cache(&self, configs: &mut Configs ) -> Result<(), Box<dyn Error>> {

        if self.content.is_some() {
            configs.buffer_factory_keeper.cache( &self.content.as_ref().unwrap().artifact );
        }

        if self.messages.is_some() && self.messages.as_ref().unwrap().create.is_some(){
            configs.buffer_factory_keeper.cache( &self.messages.as_ref().unwrap().create.as_ref().unwrap().artifact );
        }

       Ok(())
    }
}

impl ArtifactCacher for MechtronConfig {
    fn cache(&self, configs: &mut Configs) -> Result<(), Box<dyn Error>> {
       configs.tron_config_keeper.cache(&self.tron.artifact);
       Ok(())
    }
}


#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MechtronConfigYaml
{
    name: String,
    wasm: ArtifactYaml,
    tron: ArtifactYaml
}

impl MechtronConfigYaml {

    pub fn from_yaml(string:&str) -> Result<Self,Box<dyn Error>>
    {
        Ok(serde_yaml::from_str(string )?)
    }

    pub fn to_config(&self, artifact: &Artifact) -> Result<MechtronConfig,Box<dyn Error>>
    {
        let default_bundle = &artifact.bundle.clone();
        return Ok( MechtronConfig {
            source: artifact.clone(),
            wasm: self.wasm.to_artifact(default_bundle)?,
            tron: TronConfigRef{ artifact: self.tron.to_artifact(default_bundle)? },
        } )
    }
}


#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TronConfigRefYaml
{
  artifact: ArtifactYaml
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TronConfigYaml
{
    kind: Option<String>,
    name: String,
    nucleus_lookup_name: Option<String>,
    content: Option<ContentConfigYaml>,
    messages: Option<MessagesConfigYaml>
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MessagesConfigYaml
{
    create: Option<CreateConfigYaml>,
    inbound: Option<InboundConfigYaml>,
    outbound: Option<Vec<OutMessageConfigYaml>>
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct InboundConfigYaml
{
    ports: Vec<PortConfigYaml>
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct OutboundConfigYaml
{
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct CreateConfigYaml
{
  artifact: ArtifactYaml
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ContentConfigYaml
{
    artifact: ArtifactYaml
}


#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct PortConfigYaml
{
    name: String,
    description: Option<String>,
    phase: Option<String>,
    artifact: ArtifactYaml
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct OutMessageConfigYaml
{
    name: String,
    artifact: ArtifactYaml
}

impl TronConfigYaml {

    pub fn from_yaml(string:&str) -> Result<Self,Box<dyn Error>>
    {
        Ok(serde_yaml::from_str(string )?)
    }

    pub fn to_config(&self, artifact: &Artifact) -> Result<TronConfig,Box<dyn Error>>
    {
        let default_bundle = &artifact.bundle.clone();




        return Ok( TronConfig{
            kind: self.kind.clone(),
            source: artifact.clone(),
            name: self.name.clone(),

            messages: match &self.messages{
            None=>Option::None,
            Some(messages)=>Option::Some( MessagesConfig{
            create: match &messages.create {
                None=>Option::None,
                Some(create)=>Option::Some(CreateMessageConfig{artifact:create.artifact.to_artifact(default_bundle)?})
            }})},
            content: match &self.content{
                Some(content)=>Option::Some( ContentConfig{ artifact: content.artifact.to_artifact(default_bundle)?} ),
                None=>Option::None,
            },
            nucleus_lookup_name: None
        } )
    }
}


pub struct SimConfig{
    source: Artifact,
    name: String,
    description: Option<String>,
    trons: Vec<SimTronConfig>
}

pub struct SimTronConfig
{
    name: Option<String>,
    artifact: Artifact,
    create: Option<SimCreateTronConfig>
}

impl ArtifactCacher for SimTronConfig{
    fn cache(&self, configs: &mut Configs) -> Result<(), Box<dyn Error+'_>> {
        configs.tron_config_keeper.cache( &self.artifact )?;
        if self.create.is_some()
        {
            configs.artifact_cache.cache( &self.create.as_ref().unwrap().data.artifact )?;
        }
        Ok(())
    }
}

pub struct SimCreateTronConfig
{
    data: DataRef
}

pub struct DataRef{
    artifact: Artifact
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct SimConfigYaml
{
    name: String,
    main: ArtifactYaml,
    description: Option<String>,
    trons: Vec<SimTronConfigYaml>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct SimTronConfigYaml
{
    name: Option<String>,
    artifact: ArtifactYaml,
    create: Option<CreateSimTronConfigYaml>
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct CreateSimTronConfigYaml
{
    data: DataRefYaml
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DataRefYaml
{
    artifact: ArtifactYaml
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
            description: self.description.clone(),
            trons: self.trons.iter().map( |t| { SimTronConfig{
                name: t.name.clone(),
                artifact: t.artifact.to_artifact(&default_artifact)?,
                create: match &t.create {
                    None => Option::None,
                    Some(c) => Option::Some( SimCreateTronConfig{
                        data: DataRef{ artifact: c.data.artifact.to_artifact(&default_artifact)? }
                    } )
                }
            }} ).collect(),
        } )
    }
}

impl ArtifactCacher for SimConfig {
    fn cache(&self, configs: &mut Configs) -> Result<(), Box<dyn Error>> {

        for tron in self.trons
        {
            tron.cache( configs )?
        }

        Ok(())
    }
}

