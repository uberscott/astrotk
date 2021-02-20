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
use crate::nucleus::{Nuclei, Nucleus, NucleiContainer};
use crate::router::{Router, LocalRouter, NetworkRouter, SharedRouter, HasNucleus};

pub struct Node<'configs> {
    pub local: Option<Arc<Local<'configs>>>,
    pub net: Arc<Network<'configs>>,
    pub cache: Arc<Cache<'configs>>
}


impl <'configs> Node<'configs> {

    pub fn new() -> Node<'static> {
        let inter_local_router = Arc::new(SharedRouter::new());
        let inter_gateway_router= Arc::new(SharedRouter::new());
        let local_router = Arc::new(LocalRouter::new(inter_local_router.clone()));
        let network_router = Arc::new(NetworkRouter::new(inter_gateway_router.clone() )  );

        let seq = Arc::new(IdSeq::new(0));
        let network = Arc::new(Network::new(network_router.clone()));
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

        let cache = Arc::new(Cache {
            wasm_store: wasm_store,
            configs: configs,
            wasms: wasms,
        });

        let local =Arc::new(Local::new(cache.clone(), seq.clone(), inter_local_router.clone()));

        inter_local_router.set_local( local.clone() );
        inter_local_router.set_remote( local_router.clone() );
        inter_gateway_router.set_local( local_router.clone() );
        inter_gateway_router.set_remote( network_router.clone() );

        let mut rtn = Node {
            cache: cache.clone(),
            local: Option::Some(local),
            net: network.clone(),
        };
        rtn
    }

    pub fn shutdown(&self) {}

    pub fn create_sim(&self) -> Result<(Id,Id), Error>
    {
        let sim_id = self.net.seq.next();
        let nucleus_id = self.local.as_ref().unwrap().nuclei.create(sim_id, Option::Some("simulation".to_string()))?;
        Ok((sim_id,nucleus_id))
    }

    pub fn create_nucleus(&self, sim_id: &Id) -> Result<Id, Error>
    {
        let nucleus_id = self.local.as_ref().unwrap().nuclei.create(sim_id.clone(), Option::Some("simulation".to_string()))?;
        Ok(nucleus_id)
    }

    pub fn send( &self, message: Message )
    {
        self.net.router().send( Arc::new(message))
    }

}

impl<'configs> Drop for Node<'configs>
{
    fn drop(&mut self) {
        self.local = Option::None;
    }
}

pub struct Local<'configs> {
    nuclei: Nuclei<'configs>,
    router: Arc<dyn Router+'static>
}


impl <'configs> NucleiContainer for Local<'configs>
{
    fn has_nucleus(&self, id: &Id) -> bool {
        self.nuclei.has_nucleus(id)
    }
}

impl <'configs> Local <'configs>{
    fn new(cache: Arc<Cache<'configs>>, seq: Arc<IdSeq>, router: Arc<dyn Router+'static>) -> Self {
        let rtn = Local {
            nuclei: Nuclei::new(cache, seq, router.clone()),
            router: router,
        };

        rtn
    }

    pub fn has_nucleus(&self, id: &Id)->bool
    {
        self.nuclei.has_nucleus(id)
    }

}

impl<'configs> Router for Local<'configs>
{
    fn send(&self, message: Arc<Message>) {
        self.router.send(message)
    }

    fn receive(&self, message: Arc<Message>) {
        let mut result = self.nuclei.get( &message.to.tron.nucleus);

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
        unimplemented!()
    }

    fn has_nucleus_remote(&self, nucleus: &Id) -> HasNucleus {
        unimplemented!()
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
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<Module, mechtron_core::error::Error> {
        let result = Module::new(&self.wasm_store, str);
        match result {
            Ok(module) => Ok(module),
            Err(e) => Err("wasm compile error".into()),
        }
    }
}
