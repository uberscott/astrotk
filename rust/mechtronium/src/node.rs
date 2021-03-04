use std::alloc::System;
use std::borrow::{Borrow, BorrowMut};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use wasmer::{Cranelift, JIT, Module, Store};

use mechtron_core::artifact::Artifact;
use mechtron_core::configs::{Configs, Keeper, NucleusConfig, Parser, SimConfig};
use mechtron_core::core::*;
use mechtron_core::id::{Id, IdSeq, TronKey};
use mechtron_core::message::{Message, MessageKind, TronLayer, To, Cycle, DeliveryMoment};

use crate::artifact::FileSystemArtifactRepository;
use crate::cache::Cache;
use crate::error::Error;
use crate::nucleus::{Nuclei, NucleiContainer, Nucleus};
use crate::router::{HasNucleus, LocalRouter, NetworkRouter, Router, SharedRouter};
use crate::simulation::SimulationBootstrap;
use crate::mechtron::CreatePayloadsBuilder;

pub struct Node<'configs> {
    pub local: Option<Arc<Local<'configs>>>,
    pub net: Arc<Network<'configs>>,
    pub cache: Arc<Cache<'configs>>,
    pub router: Arc<dyn Router+'configs>
}


impl <'configs> Node<'configs> {

    pub fn default_cache()->Arc<Cache<'static>>
    {
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

    pub fn create_source_sim(&self, config: Arc<SimConfig>) -> Result<(Id,Id), Error> {

        if self.local.is_none()
        {
            return Err("local is none".into())
        }

        let local = self.local.as_ref().unwrap().clone();
        let sim_id = local.seq().next();

println!("HERE");
        let sim_nucleus_config = local.cache().configs.nucleus.get(&CORE_NUCLEUS_SIMULATION)?;
        let sim_nucleus_id = local.create_source_nucleus(sim_id.clone(),sim_nucleus_config.clone(),Option::Some("sim".to_string()))?;

println!("HERE2");
        let simtron_config = local.cache().configs.mechtrons.get( &CORE_MECHTRON_SIMTRON )?;
        let simtron_bind = local.cache().configs.binds.get( &simtron_config.bind.artifact )?;
        {
println!("HERE3");
            let mut builder = CreatePayloadsBuilder::new(&local.cache().configs, &simtron_bind)?;
println!("HERE4");
            let mut builder = CreatePayloadsBuilder::new(&local.cache().configs, &simtron_bind)?;
            builder.set_lookup_name("simtron");
println!("HERE5");
            builder.constructor.set( &path!["sim_config_artifact"], config.source.to() );

println!("HERE6");
            let neutron_key = TronKey::new(sim_nucleus_id.clone(),Id::new(sim_nucleus_id.seq_id, 0));

            let message = Message::multi_payload(
                local.seq().clone(),
                MessageKind::Create,
                mechtron_core::message::From{
                    tron: neutron_key.clone(),
                    cycle: 0,
                    timestamp: 0,
                    layer: TronLayer::Shell
                },
                To{
                    tron: neutron_key.clone(),
                    port: "create_simtron".to_string(),
                    cycle: Cycle::Present,
                    phase: "default".to_string(),
                    delivery: DeliveryMoment::ExtraCyclic,
                    layer: TronLayer::Kernel
                },
                CreatePayloadsBuilder::payloads(&local.cache().configs,builder));
            local.send( Arc::new(message));
        }
        Ok((sim_id,sim_nucleus_id))
    }
    pub fn create_nucleus(&self, sim_id: &Id, nucleus_config: Arc<NucleusConfig>) -> Result<Id, Error>
    {
        let nucleus_id = self.local.as_ref().unwrap().sources.create(sim_id.clone(), Option::Some("simulation".to_string()), nucleus_config)?;
        Ok(nucleus_id)
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
    sources: Nuclei<'configs>,
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
    fn parse(&self, artifact: &Artifact, str: &str) -> Result<Module, mechtron_core::error::Error> {
        let result = Module::new(&self.wasm_store, str);
        match result {
            Ok(module) => Ok(module),
            Err(e) => Err("wasm compile error".into()),
        }
    }
}
