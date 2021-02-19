use std::borrow::BorrowMut;
use std::cell::Cell;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

use no_proto::NP_Factory;
use semver::Version;
use serde::{Deserialize, Serialize};
use crate::core::*;

use crate::artifact::{Artifact, ArtifactBundle, ArtifactCache, ArtifactRepository, ArtifactYaml };
use crate::error::Error;

pub struct Configs<'config> {
    pub artifacts: Arc<dyn ArtifactCache + Sync + Send>,
    pub schemas: Keeper<NP_Factory<'config>>,
    pub sims: Keeper<SimConfig>,
    pub trons: Keeper<TronConfig>,
    pub mechtrons: Keeper<MechtronConfig>
}

impl<'config> Configs<'config> {
    pub fn new(artifact_cache: Arc<dyn ArtifactCache + Sync + Send>) -> Self {
        let mut configs = Configs {
            artifacts: artifact_cache.clone(),
            schemas: Keeper::new(
                artifact_cache.clone(),
                Box::new(NP_Buffer_Factory_Parser),
                Option::None
            ),
            sims: Keeper::new(artifact_cache.clone(), Box::new(SimConfigParser), Option::Some(Box::new(SimConfigArtifactCacher{}))),
            trons: Keeper::new(artifact_cache.clone(), Box::new(TronConfigParser), Option::Some(Box::new(TronConfigArtifactCacher{}))),
            mechtrons: Keeper::new(
                artifact_cache.clone(),
                Box::new(MechtronConfigParser),
                Option::None
            ),
        };

        configs.cache_core();

        return configs;
    }

    pub fn cache( &mut self, artifact: &Artifact )->Result<(),Error>
    {
        match &artifact.kind{
            None => {
                self.artifacts.cache(artifact)?;
                Ok(())
            }
            Some(kind) => {
                match kind.as_str(){
                    "schema"=>Ok(self.schemas.cache(artifact)?),
                    "tron_config"=>{
                        self.trons.cache(artifact)?;
                        let config = self.trons.get(artifact)?;
                        for artifact in self.trons.get_cacher().as_ref().unwrap().artifacts(config)?
                        {
                            &self.cache(&artifact)?;
                        }
                        Ok(())
                    },
                    "sim_config"=>{
                        self.sims.cache(artifact)?;
                        let config = self.sims.get(artifact)?;
                        for artifact in self.sims.get_cacher().as_ref().unwrap().artifacts(config)?
                        {
                            &self.cache(&artifact)?;
                        }
                        Ok(())
                    },
                    k => Err(format!("unrecognized kind: {}",k).into())
                }
            }
        }
    }




    pub fn cache_core(&mut self)->Result<(),Error>
    {
        self.cache(&CORE_SCHEMA_EMPTY)?;
        self.cache(&CORE_SCHEMA_META_STATE)?;
        self.cache(&CORE_SCHEMA_META_CREATE)?;


        self.cache(&CORE_SCHEMA_NEUTRON_CREATE)?;
        self.cache(&CORE_SCHEMA_NEUTRON_STATE)?;

        self.cache(&CORE_TRONCONFIG_NEUTRON)?;
        self.cache(&CORE_TRONCONFIG_SIMTRON)?;
        self.cache(&CORE_SCHEMA_NUCLEUS_LOOKUP_NAME_MESSAGE)?;
        Ok(())
    }
}

pub struct Keeper<V> {
    config_cache: RwLock<HashMap<Artifact, Arc<V>>>,
    repo: Arc<dyn ArtifactCache + Send + Sync>,
    parser: Box<dyn Parser<V> + Send + Sync>,
    cacher: Option<Box<dyn Cacher<V>+ Send+Sync>>
}

impl<V> Keeper<V> {
    pub fn new(
        repo: Arc<dyn ArtifactCache + Send + Sync>,
        parser: Box<dyn Parser<V> + Send + Sync>,
        cacher: Option<Box<dyn Cacher<V> + Send + Sync>>,
    ) -> Self {
        Keeper {
            config_cache: RwLock::new(HashMap::new()),
            parser: parser,
            cacher: cacher,
            repo: repo,
        }
    }

    pub fn cache(&mut self, artifact: &Artifact) -> Result<(),Error>  {
        let mut cache = self.config_cache.write().unwrap();

        if cache.contains_key(artifact) {
            return Ok(());
        }

        self.repo.cache(&artifact)?;

        let str = self.repo.get(&artifact).unwrap();

        let value = self.parser.parse(&artifact, str.as_ref()).unwrap();
        cache.insert(artifact.clone(), Arc::new(value));
        Ok(())
    }

    pub fn get<'get>(&self, artifact: &Artifact) -> Result<Arc<V>,Error>  where V: 'get {
        let cache = self.config_cache.read()?;

        let rtn = match cache.get(&artifact)
        {
            None => return Err(format!("could not find {}",artifact.to()).into()),
            Some(rtn) =>rtn
        };

        Ok(rtn.clone())
    }

    pub fn get_cacher( &self )->&Option<Box<dyn Cacher<V> +Send+Sync>>
    {
        &self.cacher
    }
}

pub trait Parser<V> {
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<V, Error>;
}

struct NP_Buffer_Factory_Parser;

impl<'fact> Parser<NP_Factory<'fact>> for NP_Buffer_Factory_Parser {
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<NP_Factory<'fact>, Error> {
        let result = NP_Factory::new(str);
        match result {
            Ok(rtn) => Ok(rtn),
            Err(e) => Err(format!(
                "could not parse np_factory from artifact: {}",
                artifact.to()
            )
            .into()),
        }
    }
}

struct SimConfigParser;

impl Parser<SimConfig> for SimConfigParser {
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<SimConfig, Error> {
        let sim_config_yaml = SimConfigYaml::from(str)?;
        let sim_config = sim_config_yaml.to_config(artifact)?;
        Ok(sim_config)
    }
}

struct MechtronConfigParser;

impl Parser<MechtronConfig> for MechtronConfigParser {
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<MechtronConfig, Error> {
        let mechtron_config_yaml = MechtronConfigYaml::from_yaml(str)?;
        let mechtron_config = mechtron_config_yaml.to_config(artifact)?;
        Ok(mechtron_config)
    }
}

struct TronConfigParser;

impl Parser<TronConfig> for TronConfigParser {
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<TronConfig, Error> {
        let tron_config_yaml = TronConfigYaml::from_yaml(str)?;
        let tron_config = tron_config_yaml.to_config(artifact)?;
        Ok(tron_config)
    }
}

#[derive(Clone)]
pub struct MechtronConfig {
    pub source: Artifact,
    pub wasm: Artifact,
    pub tron: TronConfigRef,
}

#[derive(Clone)]
pub struct TronConfigRef {
    pub artifact: Artifact,
}

#[derive(Clone)]
pub struct TronConfig {
    pub kind: String,
    pub name: String,
    pub nucleus_lookup_name: Option<String>,
    pub source: Artifact,
    pub state: Option<StateConfig>,
    pub message: Option<MessagesConfig>,
}

struct TronConfigArtifactCacher;

impl Cacher<TronConfig> for TronConfigArtifactCacher
{
    fn artifacts(&self, config: Arc<TronConfig>  ) -> Result<Vec<Artifact>,Error> {
        let mut rtn = vec!();
        if config.state.is_some()
        {
            rtn.push( config.state.as_ref().unwrap().artifact.clone());
        }
        if config.message.is_some()
        {
            if config.message.as_ref().unwrap().create.is_some()
            {
                rtn.push(config.message.as_ref().unwrap().create.as_ref().unwrap().artifact.clone());
            }
        }

        Ok(rtn)
    }
}

struct SimConfigArtifactCacher;
impl Cacher<SimConfig> for SimConfigArtifactCacher
{
    fn artifacts(&self, source: Arc<SimConfig>) -> Result<Vec<Artifact>, Error> {
        let mut rtn = vec!();
        for tron in &source.trons
        {
            rtn.push(tron.artifact.clone() );
        }

        Ok(rtn)
    }
}

#[derive(Clone)]
pub struct MessagesConfig {
    pub create: Option<CreateMessageConfig>,
}

#[derive(Clone)]
pub struct StateConfig {
    pub artifact: Artifact,
}

#[derive(Clone)]
pub struct CreateMessageConfig {
    pub artifact: Artifact,
}

#[derive(Clone)]
pub struct InboundMessageConfig {
    pub name: String,
    pub phase: Option<String>,
    pub artifact: Vec<Artifact>,
}

#[derive(Clone)]
pub struct OutboundMessageConfig {
    pub name: String,
    pub artifact: Artifact,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MechtronConfigYaml {
    name: String,
    wasm: ArtifactYaml,
    tron: ArtifactYaml,
}

impl MechtronConfigYaml {
    pub fn from_yaml(string: &str) -> Result<Self, Error> {
        Ok(serde_yaml::from_str(string)?)
    }

    pub fn to_config(&self, artifact: &Artifact) -> Result<MechtronConfig, Error> {
        let default_bundle = &artifact.bundle.clone();
        return Ok(MechtronConfig {
            source: artifact.clone(),
            wasm: self.wasm.to_artifact(default_bundle, Option::Some("wasm"))?,
            tron: TronConfigRef {
                artifact: self.tron.to_artifact(default_bundle, Option::Some("tron_config"))?,
            },
        });
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TronConfigRefYaml {
    artifact: ArtifactYaml,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TronConfigYaml {
    kind: String,
    name: String,
    nucleus_lookup_name: Option<String>,
    state: Option<StateConfigYaml>,
    messages: Option<MessagesConfigYaml>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MessagesConfigYaml {
    create: Option<CreateConfigYaml>,
    inbound: Option<InboundConfigYaml>,
    outbound: Option<Vec<OutMessageConfigYaml>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct InboundConfigYaml {
    ports: Vec<PortConfigYaml>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct OutboundConfigYaml {}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct CreateConfigYaml {
    artifact: ArtifactYaml,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct StateConfigYaml {
    artifact: ArtifactYaml,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct PortConfigYaml {
    name: String,
    description: Option<String>,
    phase: Option<String>,
    artifact: ArtifactYaml,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct OutMessageConfigYaml {
    name: String,
    artifact: ArtifactYaml,
}

impl TronConfigYaml {
    pub fn from_yaml(string: &str) -> Result<Self, Error> {
        Ok(serde_yaml::from_str(string)?)
    }

    pub fn to_config(&self, artifact: &Artifact) -> Result<TronConfig, Error> {
        let default_bundle = &artifact.bundle.clone();

        return Ok(TronConfig {
            kind: self.kind.clone(),
            source: artifact.clone(),
            name: self.name.clone(),

            message: match &self.messages {
                None => Option::None,
                Some(messages) => Option::Some(MessagesConfig {
                    create: match &messages.create {
                        None => Option::None,
                        Some(create) => Option::Some(CreateMessageConfig {
                            artifact: create.artifact.to_artifact(default_bundle, Option::Some("schema"))?,
                        }),
                    },
                }),
            },
            state: match &self.state {
                Some(state) => Option::Some(StateConfig {
                    artifact: state.artifact.to_artifact(default_bundle, Option::Some("schema"))?,
                }),
                None => Option::None,
            },
            nucleus_lookup_name: None,
        });
    }
}


#[derive(Clone)]
pub struct SimConfig {
    pub source: Artifact,
    pub name: String,
    pub description: Option<String>,
    pub trons: Vec<SimTronConfig>,
}

#[derive(Clone)]
pub struct SimTronConfig {
    pub name: Option<String>,
    pub artifact: Artifact,
    pub create: Option<SimCreateTronConfig>,
}

#[derive(Clone)]
pub struct SimCreateTronConfig {
    data: DataRef,
}

#[derive(Clone)]
pub struct DataRef {
    artifact: Artifact,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct SimConfigYaml {
    name: String,
    description: Option<String>,
    trons: Vec<SimTronConfigYaml>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct SimTronConfigYaml {
    name: Option<String>,
    artifact: ArtifactYaml,
    create: Option<CreateSimTronConfigYaml>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct CreateSimTronConfigYaml {
    data: DataRefYaml,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DataRefYaml {
    artifact: ArtifactYaml,
}

impl SimConfigYaml {
    pub fn from(string: &str) -> Result<Self, Error> {
        Ok(serde_yaml::from_str(string)?)
    }

    pub fn to_config(&self, artifact: &Artifact) -> Result<SimConfig, Error> {
        Ok(SimConfig {
            source: artifact.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            trons: self.to_trons(artifact)?,
        })
    }

    fn to_trons(&self, artifact: &Artifact) -> Result<Vec<SimTronConfig>, Error> {
        let default_bundle = &artifact.bundle;
        let mut rtn = vec![];
        for t in &self.trons {
            rtn.push(SimTronConfig {
                name: t.name.clone(),
                artifact: t.artifact.to_artifact(&default_bundle, Option::Some("tron_config"))?,
                create: match &t.create {
                    None => Option::None,
                    Some(c) => Option::Some(SimCreateTronConfig {
                        data: DataRef {
                            artifact: c.data.artifact.to_artifact(&default_bundle, Option::Some("schema"))?,
                        }
                    })
                }
            });
        }
        return Ok(rtn);
    }
}

pub trait Cacher<V> {
    fn artifacts(&self, source: Arc<V>) -> Result<Vec<Artifact>, Error >;
}