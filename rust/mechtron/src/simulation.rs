use crate::tron::{Tron, UpdateContext, CreateContext, InitContext};
use mechtron_common::message::Message;
use no_proto::buffer::NP_Buffer;
use std::error::Error;
use std::borrow::Borrow;
use mechtron_common::artifact::{ArtifactBundle,ArtifactYaml, ArtifactCacher, ArtifactRepository, Artifact};
use serde::__private::de::IdentifierDeserializer;
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::Deref;
use std::collections::HashMap;
use crate::nucleus::Nucleus;
use std::sync::atomic::{AtomicI64, Ordering};
use crate::app::SYS;
use std::sync::Arc;
use mechtron_common::configs::{Configs, SimConfig};

pub struct Simulation<'a> {
  id: i64,
  nuclei: HashMap<i64,Nucleus<'a>>,
  nuclei_lookup: HashMap<String,i64>,
  tron_seq: AtomicI64,
  cycle: i64
}

impl <'a> Simulation<'a>{

    pub fn init(id:i64)->Self
    {
        Simulation{
            id: id,
            nuclei: HashMap::new(),
            nuclei_lookup: HashMap::new(),
            tron_seq: AtomicI64::new(0),
            cycle: 0
        }
    }

    pub fn create(&mut self, sim_config_artifact: Artifact ) -> Result<(),Box<dyn Error>>
    {
        SYS.local.sim_config_keeper.cache(&sim_config_artifact);
        let sim_config = SYS.local.sim_config_keeper.get(&sim_config_artifact )?;

        println!("sim_config.name: {}",sim_config.name );

        // cache all artifacts that will be needed
        sim_config.cache();

        // create the sim nucleus
        let nucleus_id = SYS.net.next_nucleus_id();
        self.create_nucleus(nucleus_id, Option::Some("simulation"));

        let sim_init_context = InitContext::new(
            self.tron_seq.fetch_add(1, Ordering::Relaxed),
            sim_config_artifact.clone(),
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
