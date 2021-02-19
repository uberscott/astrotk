use std::alloc::System;
use std::borrow::{Borrow, BorrowMut};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use wasmer::{Cranelift, JIT, Module, Store};

use mechtron_core::artifact::Artifact;
use mechtron_core::configs::{Configs, Keeper, Parser, SimConfig};
use mechtron_core::id::{Id, IdSeq};
use mechtron_core::message::Message;

use crate::artifact::FileSystemArtifactRepository;
use crate::cache::Cache;
use crate::error::Error;
use crate::nucleus::{Nuclei, Nucleus};
use crate::router::{GlobalRouter, LocalRouter, Router};

pub struct Node<'configs> {
    pub local: Local<'configs>,
    pub net: Arc<Network>,
    pub cache: Arc<Cache<'configs>>
}


impl <'configs> Node<'configs> {

    pub fn new<'get>() -> Node<'get> {
        let seq = Arc::new(IdSeq::new(0));
        let network = Arc::new(Network::new());
        let repo = Arc::new(FileSystemArtifactRepository::new("../../repo/"));
        let wasm_store = Arc::new(Store::new(&JIT::new(Cranelift::default()).engine()));
        let configs = Configs::new(repo.clone());
        let wasms = Keeper::new(
            repo.clone(),
            Box::new(WasmModuleParser {
                wasm_store: wasm_store.clone(),
            },
            ),
            Option::None);
        let local_router = Arc::new(LocalRouter {});

        let cache = Arc::new(Cache {
            wasm_store: wasm_store,
            configs: configs,
            wasms: wasms,
        });


        let mut rtn = Node {
            cache: cache.clone(),
            local: Local::new(cache.clone(), seq.clone(), local_router.clone()),
            net: network.clone(),
        };
        rtn
    }

    pub fn shutdown(&self) {}

    pub fn create_sim(&self ) -> Result<Id, Error>
    {
        let sim_id = self.net.seq.next();

        self.local.nuclei.create(sim_id, Option::Some("simulation".to_string()));

        Ok(sim_id)
    }
}

impl<'configs> Drop for Node<'configs>
{
    fn drop(&mut self) {
        println!("DROPING NODE!");
        self.local.destroy();
    }
}

pub struct Local<'configs> {
    nuclei: Nuclei<'configs>,
    router: Arc<dyn Router>
}

impl <'configs> Local <'configs>{
    fn new(cache: Arc<Cache<'configs>>, seq: Arc<IdSeq>, router: Arc<LocalRouter>) -> Self {
        let rtn = Local {
            nuclei: Nuclei::new(cache.clone(), seq.clone(), router.clone()),
            router: router.clone(),
        };

        rtn
    }

    pub fn destroy(&mut self)
    {
//        self.router = Option::None;
    }
}

impl<'configs> Router for Local<'configs>
{
    fn send(&self, message: Arc<Message>) {}
}

#[derive(Clone)]
pub struct NucleusContext<'context> {
    sys: Arc<Node<'context>>,
}

impl<'context> NucleusContext<'context> {
    pub fn new(sys: Arc<Node<'context>>) -> Self {
        NucleusContext { sys: sys }
    }
    pub fn sys<'get>(&'get self) -> Arc<Node<'context>> {
        self.sys.clone()
    }
}

pub struct Network {
    seq: Arc<IdSeq>,
}

pub struct WasmStuff
{
    pub wasm_store: Arc<Store>,
    pub wasms: Keeper<Module>,
}


impl Network {
    fn new() -> Self {
        Network {
            seq: Arc::new(IdSeq::new(0)),
        }
    }

    pub fn seq(&self) -> Arc<IdSeq>
    {
        self.seq.clone()
    }
}
struct WasmModuleParser {
    wasm_store: Arc<Store>,
}

impl Parser<Module> for WasmModuleParser {
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<Module, mechtron_core::error::Error> {
        let result = Module::new(&self.wasm_store, str);
        match result {
            Ok(module) => Ok(module),
            Err(e) => Err("wasm compile error".into()),
        }
    }
}
