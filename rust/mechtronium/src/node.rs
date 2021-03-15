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
use crate::network::{NodeRouter, Wire, Connection, Route, WireListener, ExternalRoute, ReportUniqueSeqPayload};
use std::fmt;

pub struct Node{
    pub id: RefCell<Option<Id>>,
    pub seq: RefCell<Option<Arc<IdSeq>>>,
    pub local: RefCell<Option<Arc<Local>>>,
    pub kind: NodeKind,
    pub cache: Arc<Cache>,
    pub router: Arc<NodeRouter>,
}


impl Node {


    pub fn new(kind: NodeKind, cache: Option<Arc<Cache>>) -> Self {

        let cache = match cache{
            None => {
                default_cache()
            }
            Some(cache) => cache
        };


        let mut rtn = Node {
            kind: kind,
            id: RefCell::new(Option::None),
            seq: RefCell::new(Option::None),
            local: RefCell::new(Option::None),
            cache: cache.clone(),
            router: Arc::new(NodeRouter::new())
        };

        if rtn.kind.is_central()
        {
           rtn.init_with_sequence(0);
        }

        rtn
    }

    pub fn id(&self)->Id
    {
        self.id.borrow().as_ref().expect("this node has not been initialized.").clone()
    }

    pub fn seq(&self)->Arc<IdSeq>
    {
        self.seq.borrow().as_ref().expect("this node has not been initialized.").clone()
    }

    fn local(&self)->Arc<Local>
    {
        self.local.borrow().as_ref().expect("this node has not been initialized.").clone()
    }

    pub fn is_init(&self)->bool
    {
        self.id.borrow().is_some()
    }

    fn init_with_sequence(&self, seq: i64 )->Result<(),Error>
    {
        if self.id.borrow().is_some()
        {
            return Err("Node id is already set and cannot be modified!".into());
        }
        self.id.replace(Option::Some(Id::new(seq,0)));
        let seq = Arc::new(IdSeq::with_seq_and_start_index(seq,1 ));
        self.seq.replace(Option::Some(seq.clone()));
        self.local.replace(Option::Some(Arc::new(Local::new(self.cache.clone(), seq.clone(), self.router.clone()))));
        Ok(())
    }


    pub fn shutdown(&self) {}

    pub fn create_sim_from_scratch(&self, config: Arc<SimConfig>) -> Result<Id, Error> {

        if self.local.borrow().is_none()
        {
            return Err("local is none".into())
        }

        let id = self.local.borrow().as_ref().unwrap().sources.create_sim(config)?;

       Ok(id)
    }

    pub fn send( &self, message: Message )
    {
        unimplemented!()
     //   self.internal_router.send( Arc::new(message))
    }

}


pub struct Local {
    sources: Arc<Nuclei>,
    router: Arc<dyn Route>,
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
    fn new(cache: Arc<Cache>, seq: Arc<IdSeq>, router: Arc<dyn Route>) -> Self {
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
        self.router.relay(message);
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


/*
#[derive(Clone)]
pub struct NucleusContext{
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
 */


pub struct WasmStuff
{
    pub wasm_store: Arc<Store>,
    pub wasms: Keeper<Module>,
}



impl WireListener for Node
{
    fn on_wire(&self, wire: Wire, mut connection: Arc<Connection>) -> Result<(), Error> {

        match wire{
            Wire::ReportVersion(_)=> {
                // if we have a Node Id, return it
                if self.id.borrow().is_some()
                {
                    connection.relay(Wire::ReportNodeId(self.id()));
                }
                else
                {
                    connection.relay(Wire::RequestUniqueSeq);
                }
            },
            Wire::RequestUniqueSeq => {

                if self.kind.is_central()
                {
                    connection.relay( Wire::ReportUniqueSeq(ReportUniqueSeqPayload{ seq: self.seq().next().id }));
                }
                else {

                }

            }
            Wire::ReportUniqueSeq(payload) => {
                self.init_with_sequence(payload.seq);
            }
            Wire::ReportNodeId(node_id) => {
                self.router.add_external_connection( node_id, connection);
            }
            Wire::NodeSearch(search) => {

            }
            Wire::NodeFound(search) => {
            }
            Wire::MessageTransport(transport) => {
            }
            Wire::Panic(_) => {}
            _ => {
                return Err("don't know how to hanle this Wire.".into());
            }
        }
        Ok(())
    }
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

pub enum NodeKind
{
    Central(Cluster),
    Server,
    Mesh,
    Gateway,
    Client
}

impl NodeKind
{
    fn create_id(&self)->Option<Id>
    {
       match self{
           NodeKind::Central(cluster) => Option::Some(Id::new(0, 0)),
           _ => Option::None
       }
    }

    fn is_central(&self)->bool
    {
        match self{
            NodeKind::Central(_) => true,
            _ => false
        }
    }
}

impl fmt::Display for NodeKind{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let r = match self {
            NodeKind::Central(_) => {"Central"}
            NodeKind::Server => {"Server"}
            NodeKind::Mesh => {"Mesh"}
            NodeKind::Gateway => {"Gateway"}
            NodeKind::Client => {"Client"}
        };
        write!(f, "{}",r)
    }
}