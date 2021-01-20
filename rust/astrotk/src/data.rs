use std::borrow::Borrow;
use no_proto::error::NP_Error;
use no_proto::NP_Factory;
use no_proto::collection::table::NP_Table;
use no_proto::buffer::NP_Buffer;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use no_proto::pointer::{NP_Value, NP_Scalar};
use wasmer::{Module, Store, Cranelift, JIT};
use crate::artifact::Repository;
use astrotk_config::artifact_config::{ArtifactFile, ArtifactRepository, ArtifactCacher};
use astrotk_config::actor_config::{ActorConfig,ActorConfigYaml};
use std::sync::{Arc, Mutex};
use std::ops::Deref;

struct TK_Buffer_Struct<'a>
{
    buffer: NP_Buffer<'a>
}

pub trait TK_Buffer
{
}

pub struct AstroTK<'a>
{
    buffer_factories : HashMap<String, NP_Factory<'a>>,
    wasm_modules : HashMap<ArtifactFile, Module>,
    wasm_store: Store,
    pub artifact_repository: Repository,
    actor_configs: HashMap<ArtifactFile,ActorConfig>
}

impl <'a> AstroTK <'a> {

  pub fn create_buffer(&mut self, name:&str)->NP_Buffer{
      return self.get_buffer_factory(name).empty_buffer(None);
  }

  pub fn get_buffer_factory(&mut self, name:&str)->&NP_Factory
  {
      if ! self.buffer_factories.contains_key(name )
      {
          let mut file = File::open(name).expect("Unable to open file");

          let mut contents = String::new();

          file.read_to_string(&mut contents)
              .expect("Unable to read file");

          let factory = NP_Factory::new(contents).unwrap();
          self.buffer_factories.insert(name.to_string(), factory);
      }

      return &self.buffer_factories.get(name ).unwrap();
  }

    pub fn load_wasm_module(&mut self, artifact_file:&ArtifactFile) -> Result<(),Box<std::error::Error>>
    {
      if self.wasm_modules.contains_key(artifact_file)
      {
          return Ok(());
      }
      let data = self.artifact_repository.load_file(artifact_file)?;
      let module = Module::new( &self.wasm_store, data )?;
      self.wasm_modules.insert(artifact_file.clone(), module);
      return Ok(());
    }

    pub fn get_wasm_module(&self, artifact_file:&ArtifactFile)->Option<&Module>
    {
        return self.wasm_modules.get(artifact_file);
    }

    pub fn load_actor_config(&mut self, artifact_file: &ArtifactFile) -> Result<(),Box<std::error::Error>>
    {
        if self.actor_configs.contains_key(artifact_file )
        {
            return Ok(());
        }
        else
        {
            self.artifact_repository.fetch_artifact(&artifact_file.artifact)?;
            println!("{:?}", artifact_file);
            self.artifact_repository.cache_file_as_string(artifact_file)?;
            let string = self.artifact_repository.get_cached_string(artifact_file).unwrap();
            let actor_config_yaml  = ActorConfigYaml::from_yaml(string)?;
            let actor_config = actor_config_yaml.to_actor_config(artifact_file)?;
            self.load_wasm_module(&actor_config.wasm);
            actor_config.cache(&mut self.artifact_repository)?;
            self.actor_configs.insert(artifact_file.clone(), actor_config );
            return Ok(());
        }
    }

    pub fn get_actor_config(&self, artifact_file: &ArtifactFile ) -> Option<&ActorConfig>
    {
        return self.actor_configs.get(artifact_file );
    }

}

pub fn new()->AstroTK<'static>
{
    return AstroTK {
        buffer_factories: HashMap::new(),
        wasm_modules: HashMap::new(),
        wasm_store: Store::new(&JIT::new(Cranelift::default()).engine()),
        artifact_repository: Repository::new( "../../repo/".to_string() ),
        actor_configs: HashMap::new()
    }
}




