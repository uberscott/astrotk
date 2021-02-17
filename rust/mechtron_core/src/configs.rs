use std::borrow::BorrowMut;
use std::cell::Cell;
use std::collections::HashMap;
use std::error::Error;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

use no_proto::NP_Factory;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::artifact::{Artifact, ArtifactBundle, ArtifactCache, ArtifactRepository, ArtifactYaml};
use crate::error::MechError;

pub struct Configs<'config> {
    pub artifact_cache: Arc<dyn ArtifactCache + Sync + Send>,
    pub buffer_factory_keeper: Keeper<NP_Factory<'config>>,
    pub sim_config_keeper: Keeper<SimConfig>,
    pub tron_config_keeper: Keeper<TronConfig>,
    pub mechtron_config_keeper: Keeper<MechtronConfig>,
}

impl<'config> Configs<'config> {
    pub fn new(artifact_cache: Arc<dyn ArtifactCache + Sync + Send>) -> Self {
        let mut configs = Configs {
            artifact_cache: artifact_cache.clone(),
            buffer_factory_keeper: Keeper::new(
                artifact_cache.clone(),
                Box::new(NP_Buffer_Factory_Parser),
            ),
            sim_config_keeper: Keeper::new(artifact_cache.clone(), Box::new(SimConfigParser)),
            tron_config_keeper: Keeper::new(artifact_cache.clone(), Box::new(TronConfigParser)),
            mechtron_config_keeper: Keeper::new(
                artifact_cache.clone(),
                Box::new(MechtronConfigParser),
            ),
        };

        return configs;
    }
}

pub struct Keeper<V> {
    config_cache: RwLock<HashMap<Artifact, Arc<V>>>,
    repo: Arc<dyn ArtifactCache + Send + Sync>,
    parser: Box<dyn Parser<V> + Send + Sync>,
}

impl<V> Keeper<V> {
    pub fn new(
        repo: Arc<dyn ArtifactCache + Send + Sync>,
        parser: Box<dyn Parser<V> + Send + Sync>,
    ) -> Self {
        Keeper {
            config_cache: RwLock::new(HashMap::new()),
            parser: parser,
            repo: repo,
        }
    }

    pub fn cache(&mut self, artifact: &Artifact) -> Result<(),Box<dyn Error+'_>>  {
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

    pub fn get<'get>(&self, artifact: &Artifact) -> Result<Arc<V>,Box<dyn Error+'_>>  where V: 'get {
        let cache = self.config_cache.read()?;

        let rtn = match cache.get(&artifact)
        {
            None => return Err(format!("could not find {}",artifact.to()).into()),
            Some(rtn) =>rtn
        };

        Ok(rtn.clone())
    }
}

pub trait Parser<V> {
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<V, Box<dyn Error>>;
}

struct NP_Buffer_Factory_Parser;

impl<'fact> Parser<NP_Factory<'fact>> for NP_Buffer_Factory_Parser {
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<NP_Factory<'fact>, Box<dyn Error>> {
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
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<SimConfig, Box<dyn Error>> {
        let sim_config_yaml = SimConfigYaml::from(str)?;
        let sim_config = sim_config_yaml.to_config(artifact)?;
        Ok(sim_config)
    }
}

struct MechtronConfigParser;

impl Parser<MechtronConfig> for MechtronConfigParser {
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<MechtronConfig, Box<dyn Error>> {
        let mechtron_config_yaml = MechtronConfigYaml::from_yaml(str)?;
        let mechtron_config = mechtron_config_yaml.to_config(artifact)?;
        Ok(mechtron_config)
    }
}

struct TronConfigParser;

impl Parser<TronConfig> for TronConfigParser {
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
    pub content: Option<ContentConfig>,
    pub messages: Option<MessagesConfig>,
}

#[derive(Clone)]
pub struct MessagesConfig {
    pub create: Option<CreateMessageConfig>,
}

#[derive(Clone)]
pub struct ContentConfig {
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
    pub fn from_yaml(string: &str) -> Result<Self, Box<dyn Error>> {
        Ok(serde_yaml::from_str(string)?)
    }

    pub fn to_config(&self, artifact: &Artifact) -> Result<MechtronConfig, Box<dyn Error>> {
        let default_bundle = &artifact.bundle.clone();
        return Ok(MechtronConfig {
            source: artifact.clone(),
            wasm: self.wasm.to_artifact(default_bundle)?,
            tron: TronConfigRef {
                artifact: self.tron.to_artifact(default_bundle)?,
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
    content: Option<ContentConfigYaml>,
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
pub struct ContentConfigYaml {
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
    pub fn from_yaml(string: &str) -> Result<Self, Box<dyn Error>> {
        Ok(serde_yaml::from_str(string)?)
    }

    pub fn to_config(&self, artifact: &Artifact) -> Result<TronConfig, Box<dyn Error>> {
        let default_bundle = &artifact.bundle.clone();

        return Ok(TronConfig {
            kind: self.kind.clone(),
            source: artifact.clone(),
            name: self.name.clone(),

            messages: match &self.messages {
                None => Option::None,
                Some(messages) => Option::Some(MessagesConfig {
                    create: match &messages.create {
                        None => Option::None,
                        Some(create) => Option::Some(CreateMessageConfig {
                            artifact: create.artifact.to_artifact(default_bundle)?,
                        }),
                    },
                }),
            },
            content: match &self.content {
                Some(content) => Option::Some(ContentConfig {
                    artifact: content.artifact.to_artifact(default_bundle)?,
                }),
                None => Option::None,
            },
            nucleus_lookup_name: None,
        });
    }
}

pub struct SimConfig {
    pub source: Artifact,
    pub name: String,
    pub description: Option<String>,
    pub trons: Vec<SimTronConfig>,
}

pub struct SimTronConfig {
    pub name: Option<String>,
    pub artifact: Artifact,
    pub create: Option<SimCreateTronConfig>,
}

pub struct SimCreateTronConfig {
    data: DataRef,
}

pub struct DataRef {
    artifact: Artifact,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct SimConfigYaml {
    name: String,
    main: ArtifactYaml,
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
    pub fn from(string: &str) -> Result<Self, Box<dyn Error>> {
        Ok(serde_yaml::from_str(string)?)
    }

    pub fn to_config(&self, artifact: &Artifact) -> Result<SimConfig, Box<dyn Error>> {
        Ok(SimConfig {
            source: artifact.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            trons: self.to_trons(artifact)?,
        })
    }

    fn to_trons(&self, artifact: &Artifact) -> Result<Vec<SimTronConfig>, Box<dyn Error>> {
        let default_bundle = &artifact.bundle;
        let mut rtn = vec![];
        for t in &self.trons {
            rtn.push(SimTronConfig {
                name: t.name.clone(),
                artifact: t.artifact.to_artifact(&default_bundle)?,
                create: match &t.create {
                    None => Option::None,
                    Some(c) => Option::Some(SimCreateTronConfig {
                        data: DataRef {
                            artifact: c.data.artifact.to_artifact(&default_bundle)?,
                        },
                    }),
                },
            });
        }
        return Ok(rtn);
    }
}
