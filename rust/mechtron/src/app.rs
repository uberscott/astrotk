use std::borrow::Borrow;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicI64, Ordering};

use no_proto::buffer::NP_Buffer;
use no_proto::collection::table::NP_Table;
use no_proto::error::NP_Error;
use no_proto::NP_Factory;
use no_proto::pointer::{NP_Scalar, NP_Value};
use wasmer::{Cranelift, JIT, Module, Store};

use mechtron_common::artifact::{Artifact, ArtifactCacher, ArtifactRepository, Repository};
use mechtron_common::buffers::BufferFactories;
use mechtron_common::mechtron_config::{MechtronConfig, MechtronConfigYaml};

use crate::nucleus::Nucleus;
use crate::simulation::Simulation;
use std::{time, thread};

pub struct App<'a>
{
    buffer_factories: HashMap<Artifact, NP_Factory<'a>>,
    wasm_modules: HashMap<Artifact, Module>,
    wasm_store: Store,
    pub artifact_repository: Repository,
    mechtron_configs: HashMap<Artifact, MechtronConfig>,
    simulation_index: AtomicI64,
    nucleus_index: AtomicI64,
    simulations: Vec<Simulation>,
}

impl<'a> App<'a> {
    pub fn new() -> Self
    {
        return App {
            buffer_factories: HashMap::new(),
            wasm_modules: HashMap::new(),
            wasm_store: Store::new(&JIT::new(Cranelift::default()).engine()),
            artifact_repository: Repository::new("../../repo/".to_string()),
            mechtron_configs: HashMap::new(),
            simulation_index: AtomicI64::new(0),
            nucleus_index: AtomicI64::new(0),
            simulations: vec!(),
        };
    }

    pub fn create_source_simulation(&mut self, sim_config_artifact: &str) -> Result<(), Box<dyn Error>>
    {
        let sim_config_artifact = Artifact::from(sim_config_artifact)?;
        let sim_id = self.next_simulation_id();
        let mut simulation = Simulation::init(sim_id);
        simulation.create(self, sim_config_artifact)?;
        self.simulations.push(simulation);
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>>
    {
        let ten_millis = time::Duration::from_millis(100);
        let now = time::Instant::now();

        for i in 0..1000 {
            println!("update...");
            thread::sleep(ten_millis);
        }

        Ok(())
    }

    pub fn next_simulation_id(&mut self) -> i64
    {
        self.simulation_index.fetch_add(1, Ordering::Relaxed)
    }

    pub fn next_nucleus_id(&mut self) -> i64
    {
        self.nucleus_index.fetch_add(1, Ordering::Relaxed)
    }

    pub fn load_buffer_factory(&mut self, artifact_file: &Artifact) -> Result<(), Box<dyn std::error::Error>>
    {
        if self.buffer_factories.contains_key(artifact_file)
        {
            return Ok(());
        }
        self.artifact_repository.cache_file_as_string(artifact_file);
        let schema_option = self.artifact_repository.get_cached_string(artifact_file);
        if schema_option.is_none() {
            return Err(format!("cannot find cached string for buffer factory {}", artifact_file.to()).into());
        }
        let schema = schema_option.unwrap();
        let factory_result = NP_Factory::new(schema);
        if factory_result.is_err()
        {
            return Err(factory_result.err().unwrap().message.into());
        }
        let factory = factory_result.unwrap();
        self.buffer_factories.insert(artifact_file.clone(), factory);
        return Ok(());
    }


    pub fn load_wasm_module(&mut self, artifact_file: &Artifact) -> Result<(), Box<dyn std::error::Error>>
    {
        if self.wasm_modules.contains_key(artifact_file)
        {
            return Ok(());
        }
        let data = self.artifact_repository.load_file(artifact_file)?;
        let module = Module::new(&self.wasm_store, data)?;
        self.wasm_modules.insert(artifact_file.clone(), module);
        return Ok(());
    }

    pub fn get_wasm_module(&self, artifact_file: &Artifact) -> Option<&Module>
    {
        return self.wasm_modules.get(artifact_file);
    }

    pub fn load_actor_config(&mut self, artifact_file: &Artifact) -> Result<(), Box<dyn std::error::Error>>
    {
        if self.mechtron_configs.contains_key(artifact_file)
        {
            return Ok(());
        } else {
            self.artifact_repository.fetch_artifact_bundle(&artifact_file.bundle)?;
            println!("{:?}", artifact_file);
            self.artifact_repository.cache_file_as_string(artifact_file)?;
            let string = self.artifact_repository.get_cached_string(artifact_file).unwrap();
            let actor_config_yaml = MechtronConfigYaml::from_yaml(string)?;
            let actor_config = actor_config_yaml.to_actor_config(artifact_file)?;
            self.load_wasm_module(&actor_config.wasm);
            actor_config.cache(&mut self.artifact_repository)?;
            for a in actor_config.message_artifact_files() {
                self.load_buffer_factory(&a)?;
            }
            self.mechtron_configs.insert(artifact_file.clone(), actor_config);
            return Ok(());
        }
    }

    pub fn get_actor_config(&self, artifact_file: &Artifact) -> Option<&MechtronConfig>
    {
        return self.mechtron_configs.get(artifact_file);
    }
}

impl<'a> BufferFactories for App<'a>
{
    fn create_buffer(&self, artifact_file: &Artifact) -> Result<NP_Buffer, Box<dyn std::error::Error>> {
        let factory_option = self.get_buffer_factory(artifact_file);

        if factory_option.is_none() {
            return Err(format!("could not find {}", artifact_file.to()).into());
        }

        let factory = factory_option.unwrap();
        let buffer = factory.empty_buffer(Option::None);
        return Ok(buffer);
    }

    fn create_buffer_from(&self, artifact_file: &Artifact, array: Vec<u8>) -> Result<NP_Buffer, Box<dyn Error>> {
        let factory_option = self.get_buffer_factory(artifact_file);

        if factory_option.is_none() {
            return Err(format!("could not find {}", artifact_file.to()).into());
        }

        let factory = factory_option.unwrap();
        let buffer = factory.open_buffer(array);
        return Ok(buffer);
    }


    fn get_buffer_factory(&self, artifact_file: &Artifact) -> Option<&NP_Factory>
    {
        return self.buffer_factories.get(artifact_file);
    }
}





