use crate::tron::{Tron, UpdateContext, CreateContext, InitContext};
use mechtron_common::message::Message;
use no_proto::buffer::NP_Buffer;
use std::error::Error;
use std::borrow::Borrow;
use mechtron_common::artifact::{ArtifactBundle,ArtifactYaml, ArtifactCacher, ArtifactRepository, Artifact};
use crate::app::App;
use serde::__private::de::IdentifierDeserializer;
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::Deref;
use std::collections::HashMap;
use crate::nucleus::Nucleus;
use std::sync::atomic::{AtomicI64, Ordering};

pub struct Simulation {
  id: i64,
  nuclei: HashMap<i64,Nucleus>,
  nuclei_lookup: HashMap<String,i64>,
  tron_index : AtomicI64,
  cycle: i64
}

impl Simulation {

    pub fn init(id:i64)->Self
    {
        Simulation{
            id: id,
            nuclei: HashMap::new(),
            nuclei_lookup: HashMap::new(),
            tron_index: AtomicI64::new(0),
            cycle: 0
        }
    }

    pub fn create(&mut self, app: &mut App, sim_config_artifact: Artifact ) -> Result<(),Box<dyn Error>>
    {
        app.artifact_repository.cache_file_as_string(&sim_config_artifact)?;
        let sim_config = SimConfig::from(&sim_config_artifact,app)?;

        println!("sim_config.name: {}",sim_config.name );

        // cache all artifacts that will be needed
        sim_config.cache(&mut app.artifact_repository );

        // create the sim nucleus
        let nucleus_id = app.next_nucleus_id();
        self.create_nucleus(nucleus_id, Option::Some("simulation"));

        let sim_init_context = InitContext::new(
         self.tron_index.fetch_add(1,Ordering::Relaxed),
             app,
sim_config_artifact.clone()
        );

        let sim = SimTron::init(sim_init_context)?;

        self.bind_to_nucleus(nucleus_id,sim, Option::Some("simtron"));

        Ok(())
    }

    pub fn create_nucleus( &mut self, nucleus_id: i64, lookup_name: Option<&str>) -> Result<(),Box<dyn Error>>
    {
        let nucleus = Nucleus::new(self.id,nucleus_id);
        self.nuclei.insert(nucleus_id,nucleus);

        if lookup_name.is_some(){
            self.nuclei_lookup.insert( lookup_name.unwrap().to_string(), nucleus_id );
        }

        Ok(())
    }

    pub fn lookup_nucleus( &self, name: &str ) -> Option<i64>
    {
        match self.nuclei_lookup.get(name)
        {
            None => Option::None,
            Some(v) => Option::Some(v.clone())
        }
    }

    pub fn bind_to_nucleus( &mut self, nucleus_id: i64, tron: Box<dyn Tron>, lookup_name:Option<&str> ) -> Result<(),Box<dyn Error>>
    {
        let tron_id = tron.id();
        let mut nucleus = self.nuclei.get_mut(&nucleus_id );
        let mut nucleus = match nucleus {
            Some(r)=>r,
            None=>return Err(format!("could not find nucleus_id {}", nucleus_id).into())
        };
        nucleus.add(tron.id(), tron );

        if lookup_name.is_some(){
            nucleus.bind_lookup_name(tron_id, lookup_name.unwrap() );
        }

        Ok(())
    }

    pub fn cycle(&self)->i64
    {
        return self.cycle;
    }
}


pub struct SimTron
{
    id: i64,
    config: SimConfig
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

impl SimConfig {
    pub fn from(artifact_file:&Artifact, app: &App) -> Result<Self,Box<dyn Error>>
    {
        let sim_config_str = app.artifact_repository.get_cached_string(artifact_file);
        let sim_config_str = match sim_config_str{
            Some(str)=>str.to_string(),
            None=>return Err("could not get cached string".into())
        };
        let sim_config_yaml = SimConfigYaml::from(&sim_config_str)?;
        let sim_config = sim_config_yaml.to_config(artifact_file)?;
        Ok(sim_config)
    }
}

impl SimConfigYaml
{
    pub fn from(string:&str) -> Result<Self,Box<dyn Error>>
    {
        Ok(serde_yaml::from_str(string )?)
    }

    pub fn to_config(&self, artifact_file: &Artifact ) -> Result<SimConfig,Box<dyn Error>>
    {
        let default_artifact = &artifact_file.bundle.clone();
        Ok( SimConfig{
            source: artifact_file.clone(),
            name: self.name.clone(),
            main: self.main.to_artifact_file(default_artifact)?,
            create_message: self.create_message.to_artifact_file(default_artifact)?
        } )
    }
}

impl ArtifactCacher for SimConfig {

    fn cache(&self, repo: &mut dyn ArtifactRepository) -> Result<(),Box<dyn Error>>
    {
        repo.cache_file_as_string(&self.source)?;
        repo.cache_file_as_string(&self.main)?;
        repo.cache_file_as_string(&self.create_message)?;
        Ok(())
    }
}

impl Tron for SimTron{
    fn id(&self) -> i64 {
        self.id
    }

    fn init(context: InitContext) -> Result<Box<Self>, Box<dyn Error>> {
       let sim_config = SimConfig::from(context.config_artifact(), context.app)?;
       Ok(Box::new(SimTron{id:context.id,config:sim_config}))
    }

    fn create<'a,'buffer> (&self, context: &dyn CreateContext, create_message: &Message<'a>) -> Result<(NP_Buffer<'buffer>, Vec<Message<'a>>), Box<dyn Error>> {
        unimplemented!()
    }

    fn update<'a,'buffer>(  &self,
                            context: &dyn UpdateContext,
                            content: &NP_Buffer<'buffer>,
                            messages: Vec<&Message<'a>>) -> Result<(NP_Buffer<'buffer>, Vec<Message<'a>>), Box<dyn Error>>
    {
        unimplemented!()
    }
}
