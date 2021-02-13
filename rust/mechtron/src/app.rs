use std::{thread, time};
use std::borrow::Borrow;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display};
use std::fs::File;
use std::io::prelude::*;
use std::ops::Deref;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicI64, Ordering};

use no_proto::buffer::NP_Buffer;
use no_proto::error::NP_Error;
use no_proto::NP_Factory;
use no_proto::pointer::{NP_Scalar, NP_Value};
use wasmer::{CompileError, Cranelift, JIT, Module, Store};

use mechtron_common::artifact::{Artifact, ArtifactCacher};
use mechtron_common::buffers::BufferFactories;
use mechtron_common::configs::{Configs, Keeper, MechtronConfig, MechtronConfigYaml, Parser, SimConfig};
use mechtron_common::message::Message;

use crate::content::{NucleusContentStructure, TronKey};
use crate::nucleus::{NucleiStore, Nucleus};
use crate::repository::FileSystemArtifactRepository;
use crate::source::Nucleus;
use mechtron_common::id::{IdSeq, Id};
use crate::message::GlobalMessageRouter;

lazy_static! {
  pub static ref SYS : System = System::new();
}

pub struct System {
    pub local: Local,
    pub net: Network,
    pub router: GlobalMessageRouter
}

impl System
{
    fn new() -> Self {
        System {
            local: Local::new(),
            net: Network::new(),
            router: GlobalMessageRouter{}

        }
    }

    pub fn local(&mut self)->&mut Local
    {
        &mut self.local
    }

    pub fn net(&mut self)->&mut Network
    {
        &mut self.net
    }
}

pub struct Local
{
    pub wasm_store: Arc<Store>,
    pub configs: Configs,
    pub wasm_module_keeper: Keeper<Module>,
    pub nuclei: Nuclei
}

impl Local {
    fn new() -> Self
    {
        let repo = Arc::new(FileSystemArtifactRepository::new("../../repo/".to_string()));
        let wasm_store =Arc::new(Store::new(&JIT::new(Cranelift::default()).engine()));

        Local {
            wasm_store: wasm_store.clone(),
            configs: Configs::new(repo.clone()),
            wasm_module_keeper: Keeper::new(repo.clone(), Box::new(WasmModuleParser { wasm_store: wasm_store.clone() })),
            nuclei: Nuclei::new()
        }
    }
}

pub struct Network
{
    pub id_seq: IdSeq
}

impl Network
{
    fn new() -> Self
    {
        Network {
            id_seq: IdSeq::new(0)
        }
    }
}


struct WasmModuleParser
{
    wasm_store: Arc<Store>
}

impl Parser<Module> for WasmModuleParser
{
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<Module, Box<dyn Error>> {
        let result = Module::new(&self.wasm_store, str);
        match result {
            Ok(module) => Ok(module),
            Err(e) => { Err(e.into()) }
        }
    }
}




struct Nuclei
{
    sources: RwLock<HashMap<Id,Arc<Nucleus>>>
}


impl Nuclei
{
    pub fn new()->Self
    {
        Nuclei {
            sources: RwLock::new(HashMap::new() )
        }
    }

    pub fn get(&self, nucleus_id: &Id ) -> Result<Arc<Nucleus>,Box<dyn Error+'_>>
    {
        let sources = self.sources.read()?;
        if !sources.contains_key(nucleus_id)
        {
            return Err(format!("nucleus id {:?} is not present in the local nuclei", nucleus_id).into());
        }

        let nucleus= sources.get(nucleus_id).unwrap();
        return Ok(nucleus.clone());
    }

    pub fn add( &mut self, nucleus: Nucleus  )
    {
        let mut sources = self.sources.write()?;
        sources.insert(nucleus.id.clone(), Arc::new(nucleus));
    }
}
