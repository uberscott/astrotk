use std::alloc::System;
use std::borrow::{Borrow, BorrowMut};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};

use wasmer::{Cranelift, JIT, Module, Store};

use mechtron_common::artifact::Artifact;
use mechtron_common::core::*;
use mechtron_common::id::{Id, IdSeq, MechtronKey};
use mechtron_common::message::{Message, MessageKind, MechtronLayer, To, Cycle, DeliveryMoment, MessageTransport};

use crate::artifact::MechtroniumArtifactRepository;
use crate::cache::{Cache, default_cache};
use crate::error::Error;
use crate::nucleus::{Nuclei, NucleiContainer, Nucleus};
use crate::router::{HasNucleus, LocalRouter, NetworkRouter, InternalRouter, SharedRouter};
use crate::simulation::Simulation;
use crate::mechtron::CreatePayloadsBuilder;
use mechtron_common::configs::{SimConfig, Configs, Keeper, NucleusConfig, Parser};
use crate::cluster::Cluster;
use crate::network::{Router, Wire, Connection, Route, WireListener};

pub struct Node<M> where M: NodeManager {
    pub id: Option<Id>,
    pub manager: M,
    pub local: Option<Arc<Local>>,
    pub net: Arc<Network>,
    pub cache: Arc<Cache>,
    pub router: Router,
}


impl<M> Node<M> where M: NodeManager {


    pub fn new(node_manager: M, cache: Option<Arc<Cache>>) -> Self {

        let cache = match cache{
            None => {
                default_cache()
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
            id: node_manager.default_id(),
            manager: node_manager,
            cache: cache.clone(),
            local: Option::Some(local),
            net: network.clone(),
            router: Router::new()
        };
        rtn
    }

    fn relay(&self, command: Wire, connection: Arc<Connection> ) ->Result<(),Error> {
        match command{
           _ => {
                return Err("don't know how to handle command".into());
            }
        }
        Ok(())
    }



    pub fn route( transport: MessageTransport )
    {

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
        unimplemented!()
     //   self.internal_router.send( Arc::new(message))
    }

}

impl <M> Route for Node<M>  where M: NodeManager
{
    fn node_id(&self) -> Id {
        self.id.expect("Node CANNOT become a route until it has been given an Id")
    }

    fn forward(&self, message_transport: MessageTransport) -> Result<(), Error> {
        unimplemented!()
    }
}


pub struct Local {
    sources: Arc<Nuclei>,
    router: Arc<dyn InternalRouter>,
    seq: Arc<IdSeq>,
    cache: Arc<Cache>
}


impl NucleiContainer for Local
{
    fn has_nucleus(&self, id: &Id) -> bool {
        self.sources.has_nucleus(id)
    }
}

impl  Local{
    fn new(cache: Arc<Cache>, seq: Arc<IdSeq>, router: Arc<dyn InternalRouter>) -> Self {
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

    pub fn cache(&self)->Arc<Cache>
    {
        self.cache.clone()
    }

    pub fn create_source_nucleus( &self, sim_id: Id, config: Arc<NucleusConfig>, lookup_name: Option<String> )->Result<Id,Error>
    {
        self.sources.create(sim_id, lookup_name, config )
    }
}

impl InternalRouter for Local
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
pub struct NucleusContext<M> where M: NodeManager {
    sys: Arc<Node<M>>,
}

impl<M> NucleusContext<M> where M: NodeManager {
    pub fn new(sys: Arc<Node<M>>) -> Self {
        NucleusContext { sys: sys }
    }
    pub fn sys<'get>(&'get self) -> Arc<Node<M>> {
        self.sys.clone()
    }
}


pub struct WasmStuff
{
    pub wasm_store: Arc<Store>,
    pub wasms: Keeper<Module>,
}

pub struct Network {
    seq: Arc<IdSeq>,
    router: Arc<NetworkRouter>,
}

impl Network {
    fn new(router: Arc<NetworkRouter>) -> Self {
        Network {
            seq: Arc::new(IdSeq::new(0)),
            router: router
        }
    }

    pub fn seq(&self) -> Arc<IdSeq>
    {
        self.seq.clone()
    }

    pub fn router(&self)->Arc<dyn InternalRouter>
    {
        self.router.clone()
    }
}

pub trait NodeManager
{
    fn default_id(&self)->Option<Id>;
}

pub struct Central
{
    cluster: Cluster,
    node: Option<Weak<Node<Central>>>
}

impl Central{

    pub fn new(cache: Arc<Cache>) -> Self
    {
        Central{
            cluster: Cluster::new(),
            node: Option::None
        }
    }
}

impl WireListener for Central
{
    fn wire(&self, wire: Wire, connection: Arc<Connection>) -> Result<(), Error> {

        match wire{
            Wire::RequestUniqueSeq => {
                connection.relay( Wire::RespondUniqueSeq(self.cluster.seq.next().id));
            }
            Wire::RespondUniqueSeq(_) => {}
            Wire::NodeSearch(search) => {
               let node = self.node.as_ref().unwrap().upgrade().unwrap();
               if search.id == node.id.unwrap()
               {
                   let mut search = search;
                   search.hops = search.hops+1;
                   connection.relay( Wire::NodeFound(search));
               }
            }
            Wire::NodeFound(search) => {
                let node = self.node.as_ref().unwrap().upgrade().unwrap();
                node.router.add_route( search, connection );
            }
            Wire::MessageTransport(transport) => {
                let node = self.node.as_ref().unwrap().upgrade().unwrap();
                node.router.forward( transport );
            }
            Wire::Panic(_) => {}
            _ => {
                return Err("don't know how to hanle this Wire.".into());
            }
        }
        Ok(())
    }
}

impl NodeManager for Central
{
    fn default_id(&self) -> Option<Id> {
        Option::Some(Id::new(0,0))
    }
}


pub struct Server
{
}



pub struct Mesh
{
}

pub struct Gateway
{
}

pub struct Client
{
}

