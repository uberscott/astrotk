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
    pub binds: Keeper<BindConfig>,
    pub nucleus: Keeper<NucleusConfig>,
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
            binds: Keeper::new(artifact_cache.clone(), Box::new(BindParser), Option::Some(Box::new(BindCacher {}))),
            nucleus: Keeper::new(artifact_cache.clone(), Box::new(NucleusConfigParser ), Option::Some(Box::new(NucleusConfigArtifactCacher{}))),
            mechtrons: Keeper::new(
                artifact_cache.clone(),
                Box::new(MechtronConfigParser),
                Option::Some(Box::new(MechtronConfigCacher))
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
                    "bind"=>{
                        self.binds.cache(artifact)?;
                        let config = self.binds.get(artifact)?;
                        for artifact in self.binds.get_cacher().as_ref().unwrap().artifacts(config)?
                        {
                            &self.cache(&artifact)?;
                        }
                        Ok(())
                    },
                    "nucleus"=>{
                        self.nucleus.cache(artifact)?;
                        let config = self.nucleus.get(artifact)?;
                        for artifact in self.nucleus.get_cacher().as_ref().unwrap().artifacts(config)?
                        {
                            &self.cache(&artifact)?;
                        }
                        Ok(())
                    },
                    "mechtron"=>{
                        self.mechtrons.cache(artifact)?;
                        let config = self.mechtrons.get(artifact)?;
                        for artifact in self.mechtrons.get_cacher().as_ref().unwrap().artifacts(config)?
                        {
                            &self.cache(&artifact)?;
                        }
                        Ok(())
                    },
                    "sim"=>{
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

        self.cache(&CORE_BIND_NEUTRON)?;
        self.cache(&CORE_BIND_SIMTRON)?;

        self.cache(&CORE_SCHEMA_EMPTY)?;
        self.cache(&CORE_SCHEMA_META_STATE)?;
        self.cache(&CORE_SCHEMA_META_CREATE)?;

        self.cache(&CORE_SCHEMA_NEUTRON_CREATE)?;
        self.cache(&CORE_SCHEMA_NEUTRON_STATE)?;

        self.cache(&CORE_SCHEMA_NUCLEUS_LOOKUP_NAME_MESSAGE)?;
        self.cache(&CORE_SCHEMA_PING)?;
        self.cache(&CORE_SCHEMA_PONG)?;
        self.cache(&CORE_SCHEMA_TEXT)?;
        self.cache(&CORE_SCHEMA_OK)?;

        self.cache(&CORE_NUCLEUS_SIMULATION)?;
        self.cache(&CORE_MECHTRON_SIMTRON)?;
        self.cache(&CORE_MECHTRON_NEUTRON)?;

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

        println!("caching: {}",artifact.to());

        self.repo.cache(&artifact)?;

        let str = self.repo.get(&artifact)?;

        let value = self.parser.parse(&artifact, str.as_ref())?;
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
                "could not parse np_factory from artifact: {} error: {:?}",
                artifact.to(), e
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

            /*
        Ok( MechtronConfig{
            source: Default::default(),
            wasm: Default::default(),
            bind: BindRef { artifact: Default::default() }
        })

             */
    }
}

struct BindParser;

impl Parser<BindConfig> for BindParser {
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<BindConfig, Error> {
        let bind_yaml = BindYaml::from_yaml(str)?;
        let bind = bind_yaml.to_config(artifact)?;
        Ok(bind)
    }
}


struct NucleusConfigParser;

impl Parser<NucleusConfig> for NucleusConfigParser {
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<NucleusConfig, Error> {
        let nucleus_config_yaml = NucleusConfigYaml::from_yaml(str)?;
        let nucleus_config = nucleus_config_yaml.to_config(artifact)?;
        Ok(nucleus_config)
    }
}


#[derive(Clone)]
pub struct MechtronConfig {
    pub source: Artifact,
    pub name: Option<String>,
    pub wasm: Artifact,
    pub bind: BindRef,
}

#[derive(Clone)]
pub struct BindRef {
    pub artifact: Artifact,
}

#[derive(Clone)]
pub struct BindConfig {
    pub kind: String,
    pub name: String,
    pub nucleus_lookup_name: Option<String>,
    pub source: Artifact,
    pub state: StateConfig,
    pub message: MessageConfig,
}

struct NucleusConfigArtifactCacher;

impl Cacher<NucleusConfig> for NucleusConfigArtifactCacher
{
    fn artifacts(&self, config: Arc<NucleusConfig>) -> Result<Vec<Artifact>, Error> {
        Ok(vec!())
    }
}

struct BindCacher;

impl Cacher<BindConfig> for BindCacher
{
    fn artifacts(&self, config: Arc<BindConfig>  ) -> Result<Vec<Artifact>,Error> {
        let mut rtn = vec!();
        let config = &config;

        rtn.push( config.state.artifact.clone());
        rtn.push(config.message.create.artifact.clone());
        for port in config.message.extra.values()
        {
            rtn.push( port.artifact.clone() );
        }
        for port in config.message.inbound.values()
        {
            rtn.push( port.artifact.clone() );
        }
        for port in config.message.outbound.values()
        {
            rtn.push( port.artifact.clone() );
        }

        Ok(rtn)
    }
}

struct SimConfigArtifactCacher;
impl Cacher<SimConfig> for SimConfigArtifactCacher
{
    fn artifacts(&self, source: Arc<SimConfig>) -> Result<Vec<Artifact>, Error> {
        let mut rtn = vec!();
/*        for tron in &source.trons
        {
            rtn.push(tron.artifact.clone() );
        }

 */

        Ok(rtn)
    }
}


struct MechtronConfigCacher;
impl Cacher<MechtronConfig> for MechtronConfigCacher
{
    fn artifacts(&self, source: Arc<MechtronConfig>) -> Result<Vec<Artifact>, Error> {
        let mut rtn = vec!();
        rtn.push( source.bind.artifact.clone() );
        Ok(rtn)
    }
}

#[derive(Clone)]
pub struct MessageConfig {
    pub create: CreateMessageConfig,
    pub extra: HashMap<String,ExtraMessageConfig>,
    pub inbound: HashMap<String,InboundMessageConfig>,
    pub outbound: HashMap<String,OutboundMessageConfig>,
}

impl Default for MessageConfig
{
    fn default() -> Self {

        MessageConfig{
            create: Default::default(),
            extra:  Default::default(),
            inbound: Default::default(),
            outbound: Default::default()
        }
    }
}

#[derive(Clone)]
pub struct NucleusConfig
{
   pub name: Option<String>,
   pub description: Option<String>,
   pub phases: Vec<PhaseConfig>,
   pub mectrons: Vec<MechtronConfigRef>,
}

#[derive(Clone)]
pub struct PhaseConfig
{
    pub name: String
}


#[derive(Clone)]
pub struct StateConfig {
    pub artifact: Artifact,
}

impl Default for StateConfig {
    fn default() -> Self {
        StateConfig{
            artifact: Default::default()
        }
    }
}

#[derive(Clone)]
pub struct CreateMessageConfig {
    pub artifact: Artifact,
}

impl Default for CreateMessageConfig
{
    fn default() -> Self {
        CreateMessageConfig{
            artifact: Default::default()
        }
    }
}

#[derive(Clone)]
pub struct ExtraMessageConfig {
    pub name: String,
    pub artifact: Artifact,
}


#[derive(Clone)]
pub struct InboundMessageConfig {
    pub name: String,
    pub artifact: Artifact,
}

#[derive(Clone)]
pub struct OutboundMessageConfig {
    pub name: String,
    pub artifact: Artifact,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MechtronConfigYaml {
    name: Option<String>,
    wasm: ArtifactYaml,
    bind: ArtifactYaml,
}

impl MechtronConfigYaml {
    pub fn from_yaml(string: &str) -> Result<Self, Error> {
        Ok(serde_yaml::from_str(string)?)
    }

    pub fn to_config(&self, artifact: &Artifact) -> Result<MechtronConfig, Error> {
        let default_bundle = &artifact.bundle.clone();
        return Ok(MechtronConfig {
            source: artifact.clone(),
            name: self.name.clone(),
            wasm: self.wasm.to_artifact(default_bundle, Option::Some("wasm"))?,
            bind: BindRef {
                artifact: self.bind.to_artifact(default_bundle, Option::Some("bind"))?,
            },
        });
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TronConfigRefYaml {
    artifact: ArtifactYaml,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct BindYaml {
    kind: String,
    name: String,
    state: Option<StateConfigYaml>,
    message: Option<MessageConfigYaml>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MessageConfigYaml {
    create: Option<CreateConfigYaml>,
    extra: Option<PortsYaml<ExtraConfigYaml>>,
    inbound: Option<PortsYaml<InboundConfigYaml>>,
    outbound: Option<PortsYaml<OutboundConfigYaml>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct PortsYaml<T>
{
    ports: Vec<T>
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtraConfigYaml {
    name: String,
    artifact: ArtifactYaml
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct InboundConfigYaml {
    name: String,
    artifact: ArtifactYaml
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct OutboundConfigYaml {
    name: String,
    artifact: ArtifactYaml,
}

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

impl BindYaml {
    pub fn from_yaml(string: &str) -> Result<Self, Error> {
        Ok(serde_yaml::from_str(string)?)
    }

    pub fn to_config(&self, artifact: &Artifact) -> Result<BindConfig, Error> {
        let default_bundle = &artifact.bundle.clone();

        return Ok(BindConfig {
            kind: self.kind.clone(),
            source: artifact.clone(),
            name: self.name.clone(),

            message: match &self.message {
                None => Default::default(),
                Some(messages) => MessageConfig {
                    create: match &messages.create {
                        None => Default::default(),
                        Some(create) => CreateMessageConfig {
                            artifact: create.artifact.to_artifact(default_bundle, Option::Some("schema"))?,
                        },
                    },
                    extra: match &messages.extra{
                        None => Default::default(),
                        Some(extras) =>
                            extras.ports.iter().map(|extra|->Result<ExtraMessageConfig,Error>{
                                Ok(ExtraMessageConfig{
                                    name: extra.name.clone(),
                                    artifact: extra.artifact.to_artifact(default_bundle, Option::Some("schema"))?
                                })
                            }).filter(|r|{
                                if r.is_err(){
                                    println!("error processing extra" )
                                }

                                r.is_ok()}).map(|r|r.unwrap()).map(|c|(c.name.clone(),c)).collect()
                    },
                    inbound: match &messages.inbound{
                        None => Default::default(),
                        Some(inbounds) =>
                            inbounds.ports.iter().map(|inbound|->Result<InboundMessageConfig,Error>{
                               Ok(InboundMessageConfig{
                                   name: inbound.name.clone(),
                                   artifact: inbound.artifact.to_artifact(default_bundle, Option::Some("schema"))?
                               })
                            }).filter(|r|{
                                if r.is_err(){
                                    println!("error processing inbound" )
                                }

                                r.is_ok()}).map(|r|r.unwrap()).map(|c|(c.name.clone(),c)).collect()
                    },
                    outbound: match &messages.outbound{
                        None => Default::default(),
                        Some(inbounds) =>
                            inbounds.ports.iter().map(|outbound|->Result<OutboundMessageConfig,Error>{
                                Ok(OutboundMessageConfig{
                                    name: outbound.name.clone(),
                                    artifact: outbound.artifact.to_artifact(default_bundle, Option::Some("schema"))?
                                })
                            }).filter(|r|

                                                                      {
                                if r.is_err(){
                                    println!("error processing outbound" )
                                }

                                r.is_ok()}).map(|r|r.unwrap()).map(|c|(c.name.clone(),c)).collect()
                    },
                },
            },
            state: match &self.state {
                Some(state) => StateConfig {
                    artifact: state.artifact.to_artifact(default_bundle, Option::Some("schema"))?,
                },
                None => Default::default(),
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
    pub nucleus: Vec<NucleusConfigRef>
}

#[derive(Clone)]
pub struct NucleusConfigRef{
    pub name: Option<String>,
    pub auto: Option<bool>,
    pub kind: Option<String>,
    pub artifact: Artifact,
}

#[derive(Clone)]
pub struct MechtronConfigRef {
    pub name: Option<String>,
    pub artifact: Artifact,
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
    nucleus: Option<Vec<NucleusConfigRefYaml>>
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct NucleusConfigRefYaml{
    name: Option<String>,
    kind: Option<String>,
    auto: Option<bool>,
    artifact: ArtifactYaml
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
            nucleus: match &self.nucleus{
                None => vec!(),
                Some(nuclei) => nuclei.iter().map( |nucleus|{
                    NucleusConfigRef{
                        name: nucleus.name.clone(),
                        kind: nucleus.kind.clone(),
                        auto: nucleus.auto.clone(),
                        artifact: nucleus.artifact.to_artifact(&artifact.bundle, Option::Some("nucleus")).unwrap()
                    }
                }).collect()
            }
        })
    }


}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct NucleusConfigYaml
{
    name: Option<String>,
    description: Option<String>,
    phases: Vec<PhaseConfigYaml>
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct PhaseConfigYaml
{
    name: String
}

impl NucleusConfigYaml
{
    pub fn from_yaml(string: &str) -> Result<Self, Error> {
        Ok(serde_yaml::from_str(string)?)
    }

    pub fn to_config(&self, artifact: &Artifact) -> Result<NucleusConfig, Error> {
        let default_bundle = &artifact.bundle.clone();
        Ok(NucleusConfig{
            name: self.name.clone(),
            description: self.description.clone(),
            phases: self.phases.iter().map( |p| PhaseConfig{ name: p.name.clone() } ).collect(),
            mectrons: vec!()
        })
    }


}

pub trait Cacher<V> {
    fn artifacts(&self, source: Arc<V>) -> Result<Vec<Artifact>, Error >;
}