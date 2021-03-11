use std::alloc::System;
use std::borrow::{Borrow, BorrowMut};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};

use wasmer::{Cranelift, JIT, Module, Store};

use mechtron_common::artifact::Artifact;
use mechtron_common::core::*;
use mechtron_common::id::{Id, IdSeq, MechtronKey};
use mechtron_common::message::{Message, MessageKind, MechtronLayer, To, Cycle, DeliveryMoment};

use crate::artifact::MechtroniumArtifactRepository;
use crate::cache::Cache;
use crate::error::Error;
use crate::nucleus::{Nuclei, NucleiContainer, Nucleus};
use crate::router::{HasNucleus, LocalRouter, NetworkRouter, Router, SharedRouter};
use crate::simulation::Simulation;
use crate::mechtron::CreatePayloadsBuilder;
use mechtron_common::configs::{SimConfig, Configs, Keeper, NucleusConfig, Parser};

pub struct Node<'configs> {
    pub local: Option<Arc<Local<'configs>>>,
    pub net: Arc<Network<'configs>>,
    pub cache: Arc<Cache<'configs>>,
    pub router: Arc<dyn Router+'configs>
}


impl <'configs> Node<'configs> {

    pub fn default_cache()->Arc<Cache<'static>>
    {
        let repo = Arc::new(MechtroniumArtifactRepository::new("../../repo/"));
        let wasm_store = Arc::new(Store::new(&JIT::new(Cranelift::default()).engine()));
        let configs = Configs::new(repo.clone());
        let wasms = Keeper::new(
            repo.clone(),
            Box::new(WasmModuleParser {
                wasm_store: wasm_store.clone(),
            },
            ),
            Option::None);

        Arc::new(Cache {
            wasm_store: wasm_store,
            configs: configs,
            wasms: wasms,
        })
    }

    pub fn new(cache: Option<Arc<Cache<'static>>>) -> Node<'static> {

        let cache = match cache{
            None => {
                Node::default_cache()
            }
            Some(cache) => cache
        };

        let inter_local_router = Arc::new(SharedRouter::new());
        let inter_gateway_router= Arc::new(SharedRouter::new());
        let local_router = Arc::new(LocalRouter::new(inter_local_router.clone()));
        let network_router = Arc::new(NetworkRouter::new(inter_gateway_router.clone() )  );

        let seq = Arc::new(IdSeq::new(0));
        let network = Arc::new(Network::new(network_router.clone()));

        let local =Arc::new(Local::new(cache.clone(), seq.clone(), inter_local_router.clone()));

        inter_local_router.set_local( local.clone() );
        inter_local_router.set_remote( local_router.clone() );
        inter_gateway_router.set_local( local_router.clone() );
        inter_gateway_router.set_remote( network_router.clone() );

        let mut rtn = Node {
            cache: cache.clone(),
            local: Option::Some(local),
            net: network.clone(),
            router: local_router.clone()
        };
        rtn
    }

    pub fn shutdown(&self) {}

    pub fn create_sim_from_scratch(&self, config: Arc<SimConfig>) -> Result<Id, Error> {

        if self.local.is_none()
        {
            return Err("local is none".into())
        }

        let id = self.local.as_ref().unwrap().sources.create_sim(config)?;

       Ok(id)
    }

    pub fn send( &self, message: Message )
    {
        self.router.send( Arc::new(message))
    }

}

impl<'configs> Drop for Node<'configs>
{
    fn drop(&mut self) {
        self.local = Option::None;
    }
}

pub struct Local<'configs> {
    sources: Arc<Nuclei<'configs>>,
    router: Arc<dyn Router+'static>,
    seq: Arc<IdSeq>,
    cache: Arc<Cache<'configs>>
}


impl <'configs> NucleiContainer for Local<'configs>
{
    fn has_nucleus(&self, id: &Id) -> bool {
        self.sources.has_nucleus(id)
    }
}

impl <'configs> Local <'configs>{
    fn new(cache: Arc<Cache<'configs>>, seq: Arc<IdSeq>, router: Arc<dyn Router+'static>) -> Self {
        let rtn = Local {
            sources: Nuclei::new(cache.clone(), seq.clone(), router.clone()),
            router: router,
            seq: seq.clone(),
            cache: cache.clone()
        };

        rtn
    }

    pub fn has_nucleus(&self, id: &Id)->bool
    {
        self.sources.has_nucleus(id)
    }

    pub fn seq(&self)->Arc<IdSeq>
    {
        self.seq.clone()
    }

    pub fn cache(&self)->Arc<Cache<'configs>>
    {
        self.cache.clone()
    }

    pub fn create_source_nucleus( &self, sim_id: Id, config: Arc<NucleusConfig>, lookup_name: Option<String> )->Result<Id,Error>
    {
        self.sources.create(sim_id, lookup_name, config )
    }
}

impl<'configs> Router for Local<'configs>
{
    fn send(&self, message: Arc<Message>) {
        self.router.send(message)
    }

    fn receive(&self, message: Arc<Message>) {
        let mut result = self.sources.get( &message.to.tron.nucleus);

        if result.is_err()
        {
            println!("cannot find nucleus with id: {:?}",message.to.tron.nucleus);
        }
        else {
            let mut nucleus = result.unwrap();
            nucleus.intake_message(message);
        }
    }

    fn has_nucleus_local(&self, nucleus: &Id) -> HasNucleus {
        match self.sources.has_nucleus(nucleus)
        {
            true => HasNucleus::Yes,
            false => HasNucleus::No
        }
    }

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


pub struct WasmStuff
{
    pub wasm_store: Arc<Store>,
    pub wasms: Keeper<Module>,
}

pub struct Network<'net> {
    seq: Arc<IdSeq>,
    router: Arc<NetworkRouter<'net>>,
}

impl <'net> Network<'net> {
    fn new(router: Arc<NetworkRouter<'net>>) -> Self {
        Network {
            seq: Arc::new(IdSeq::new(0)),
            router: router
        }
    }

    pub fn seq(&self) -> Arc<IdSeq>
    {
        self.seq.clone()
    }

    pub fn router(&self)->Arc<dyn Router+'net>
    {
        self.router.clone()
    }
}
struct WasmModuleParser {
    wasm_store: Arc<Store>,
}

impl Parser<Module> for WasmModuleParser {
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<Module, mechtron_common::error::Error> {
        let result = Module::new(&self.wasm_store, str);
        match result {
            Ok(module) => Ok(module),
            Err(e) => Err("wasm compile error".into()),
        }
    }
}
