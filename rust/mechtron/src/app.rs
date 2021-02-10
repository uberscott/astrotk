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
use no_proto::collection::table::NP_Table;
use no_proto::error::NP_Error;
use no_proto::NP_Factory;
use no_proto::pointer::{NP_Scalar, NP_Value};
use wasmer::{CompileError, Cranelift, JIT, Module, Store};

use mechtron_common::artifact::{Artifact, ArtifactCacher};
use mechtron_common::buffers::BufferFactories;
use mechtron_common::configs::{Configs, Keeper, MechtronConfig, MechtronConfigYaml, Parser};
use mechtron_common::message::Message;

use crate::content::{ContentStore, TronKey};
use crate::message::{MessageIntake, MessageRouter};
use crate::nucleus::NucleiStore;
use crate::repository::FileSystemArtifactRepository;
use crate::source::Source;

lazy_static! {
  pub static ref SYS : System = System::new();
}

pub fn sys()->&System
{
    return &SYS;
}

pub struct System {
    local: Local,
    net: Network,
}

impl System
{
    fn new() -> Self {
        System {
            local: Local::new(),
            net: Network::new()
        }
    }

    pub fn local(&self)->&Local
    {
        &self.local
    }

    pub fn net(&self)->&Network
    {
        &self.net
    }
}

pub struct Local
{
    pub wasm_store: Arc<Store>,
    pub configs: Configs,
    pub wasm_module_keeper: Keeper<Module>
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
        }
    }
}

pub struct Network
{
    nucleus_seq: AtomicI64,
    sim_seq: AtomicI64,
}

impl Network
{
    fn new() -> Self
    {
        Network {
            nucleus_seq: AtomicI64::new(0),
            sim_seq: AtomicI64::new(0),
        }
    }

    pub fn next_nucleus_id(&self) -> i64 {
       self.nucleus_seq.fetch_add(1,Ordering::Relaxed)
    }

    pub fn next_sim_id(&self) -> i64 {
        self.sim_seq.fetch_add(1,Ordering::Relaxed)
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

    pub fn add( &mut self, sim_id: i64 )->Result<(),Box<dyn Error + '_>>
    {
        let mut sources = self.sources.write()?;
        if sources.contains_key(&sim_id)
        {
           return Err(format!("sim id {} has already been added to the sources",sim_id).into());
        }

        sources.insert( sim_id, Source::new(sim_id));

        Ok(())
    }
    /*
    pub fn get( &self, sim_id: i64 ) -> Result<&'a Source,Box<dyn Error+'_>>
    {
        let sources = self.sources.read()?;
        if !sources.contains_key(&sim_id)
        {
            return Err(format!("sim id {} is not present in the sources",sim_id).into());
        }

        let source:Option<&'a Source> = sources.get(&sim_id);
        let source:&'a Source = source.unwrap();
        return Ok(source);
    }
     */

}
