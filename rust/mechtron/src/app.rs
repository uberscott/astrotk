use std::{thread, time};
use std::borrow::Borrow;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::ops::Deref;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicI64, Ordering};

use no_proto::buffer::NP_Buffer;
use no_proto::collection::table::NP_Table;
use no_proto::error::NP_Error;
use no_proto::NP_Factory;
use no_proto::pointer::{NP_Scalar, NP_Value};
use wasmer::{Cranelift, JIT, Module, Store, CompileError};

use mechtron_common::artifact::{Artifact, ArtifactCacher};
use mechtron_common::buffers::BufferFactories;
use mechtron_common::configs::{Configs, MechtronConfig, MechtronConfigYaml, Keeper, Parser};
use mechtron_common::message::Message;

use crate::message::{MessageRouter, MessageIntake};
use crate::nucleus::NucleiStore;
use crate::repository::FileSystemArtifactRepository;
use crate::content::{ContentStore, TronKey};
use crate::source::Source;

lazy_static! {
  pub static ref SYS : System<'static> = System::new();
}

pub struct System<'a> {
    pub local: Local,
    pub net: Network<'a>,
}

impl <'a> System<'a>
{
    fn new() -> Self {
        System {
            local: Local::new(),
            net: Network::new(),
        }
    }
}

pub struct Local
{
    pub wasm_store: Store,
    pub configs: Configs,
    pub wasm_module_keeper: Keeper<Module>
}

impl Local {
    fn new() -> Self
    {
        let repo = Arc::new(FileSystemArtifactRepository::new("../../repo/".to_string()));
        Local {
            wasm_store: Store::new(&JIT::new(Cranelift::default()).engine()),
            configs: Configs::new(repo.clone()),
            wasm_module_keeper: Keeper::new(repo.clone(), Box::new(WasmModuleParser)),
        }
    }
}

pub struct Network<'a>
{
    nucleus_seq: AtomicI64,
    sim_seq: AtomicI64,
    router: Mutex<Box<dyn MessageRouter<'a>>>
}

impl <'a> Network<'a>
{
    fn new() -> Self
    {
        Network {
            nucleus_seq: AtomicI64::new(0),
            sim_seq: AtomicI64::new(0),
            router: Mutex::new(Box::new(LocalMessageRouter{} ) )
        }
    }

    pub fn next_nucleus_id(&self) -> i64 {
       self.nucleus_seq.fetch_add(1,Ordering::Relaxed)
    }

    pub fn next_sim_id(&self) -> i64 {
        self.sim_seq.fetch_add(1,Ordering::Relaxed)
    }
}


struct WasmModuleParser;

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


struct LocalMessageRouter
{}

impl LocalMessageRouter
{
    pub fn new() -> Self {
        LocalMessageRouter {}
    }
}

impl MessageRouter<'_> for LocalMessageRouter
{
    fn send(&self, messages: Vec<Message>) {
        unimplemented!()
    }
}

struct Sources<'a>
{
    sources: RwLock<HashMap<i64,Source<'a>>>
}

impl <'a> Sources<'a>
{
    pub fn new()->Self
    {
        Sources{
            sources: RwLock::new(HashMap::new() )
        }
    }

    pub fn add( &mut self, sim_id: i64 )->Result<(),Box<dyn Error>>
    {
        let mut sources = self.sources.write()?;
        if sources.contains_key(&sim_id)
        {
           return Err(format!("sim id {} has already been added to the sources",sim_id).into());
        }

        sources.insert( sim_id, Source::new(sim_id));

        Ok(())
    }

    pub fn get( &self, sim_id: i64 ) -> Result<&Source,Box<dyn Error>>
    {
        let sources = self.sources.read()?;
        if !sources.contains_key(&sim_id)
        {
            return Err(format!("sim id {} is not present in the sources",sim_id).into());
        }

        let source = sources.get(&sim_id);
        return Ok(source.unwrap());
    }

}
