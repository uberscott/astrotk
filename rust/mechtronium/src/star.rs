use std::alloc::System;
use std::borrow::{Borrow, BorrowMut};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak, Mutex};

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
use crate::network::{NodeRouter, Wire, Connection, Route, WireListener, ExternalRoute, ReportUniqueSeqPayload, RelayPayload, NodeFind, Relay, ReportSupervisorForSim, ReportAssignSimulation, Search, SearchResult, SearchKind};
use std::fmt;
use crate::network::RelayPayload::ReportSupervisorAvailable;


pub struct Star {
    pub id: RefCell<Option<Id>>,
    pub seq: RefCell<Option<Arc<IdSeq>>>,
    pub local: RefCell<Option<Arc<Local>>>,
    pub core: StarCore,
    pub cache: Arc<Cache>,
    pub router: Arc<NodeRouter>,
    connection_transactions: Mutex<HashMap<Id,Arc<Connection>>>,
    pub error_handler: RefCell<Box<dyn StarErrorHandler>>
}


impl Star {


    pub fn new(kind: StarCore, cache: Option<Arc<Cache>>) -> Self {

        let cache = match cache{
            None => {
                default_cache()
            }
            Some(cache) => cache
        };


        let mut rtn = Star {
            core: kind,
            id: RefCell::new(Option::None),
            seq: RefCell::new(Option::None),
            local: RefCell::new(Option::None),
            cache: cache.clone(),
            router: Arc::new(NodeRouter::new()),
            connection_transactions: Mutex::new(HashMap::new()),
            error_handler: RefCell::new( Box::new(LogErrorHandler{}) )
        };

        if rtn.core.is_central()
        {
           rtn.init_with_sequence(0);
        }

        rtn
    }

    pub fn id(&self)->Id
    {
        self.id.borrow().as_ref().expect("this node has not been initialized.").clone()
    }

    pub fn error(&self, message: &str )
    {
        let handler = self.error_handler.borrow();
        handler.on_error(message);
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
        self.router.set_node_id(self.id());


        Ok(())
    }

    fn start(&self)
    {
        match &self.core
        {
            StarCore::Supervisor(supervisor) => {
                self.router.relay_wire(Wire::Relay(Relay::to_central(self.id(),
                                                                     ReportSupervisorAvailable)));
            },
            StarCore::Server(_) => {
                self.router.relay_wire(Wire::Search(Search::new(self.id(),
                                                                     SearchKind::StarKind(StarKind::Supervisor) )));
            },
            _ => {}
        }

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

    pub fn timestamp(&self)->u64
    {
        // implement later
        0
    }

    pub fn send( &self, message: Message )
    {
        unimplemented!()
     //   self.internal_router.send( Arc::new(message))
    }

    fn on_search( &self, search: Search, connection: Arc<Connection> )->Result<(),Error>
    {
        if search.hops < 0
        {
            self.error("Illegal search hops!");
        }
        if search.max_hops > 16
        {
            self.error("Illegal max search hops!");
        }
        else if search.hops > search.max_hops
        {
            self.error("Too many search hops!");
        }

        let search = search.inc_hops();

        if search.kind.matches(self)
        {
            let reply = Relay::new(self.id(),search.from.clone(), RelayPayload::SearchResult(SearchResult{
                found: self.id().clone(),
                kind: search.kind.clone(),
                hops: search.hops.clone(),
                transaction: Option::None
            }));
            self.router.relay_wire(Wire::Relay(reply) );
        }
        self.router.relay_wire_excluding(Wire::Search(search), connection.get_remote_node_id());

        Ok(())
    }

    fn on_relay( &self, relay: Relay, connection: Arc<Connection> )->Result<(),Error>
    {

        if relay.hops < 0
        {
            self.error("Illegal payload hops!");
        }
        else if relay.hops > 16
        {
            self.error("Too many payload hops!");
        }

        // handled the same no matter if this is the To node or not
        match &relay.payload
        {
            RelayPayload::NodeFound(search)=> {
                connection.add_found_node(search.seeking_id.clone() , NodeFind::new(search.hops,self.timestamp()));
                self.router.notify_found( &search.seeking_id );

            }
            RelayPayload::NodeNotFound(search)=> {
                connection.add_unfound_node(search.seeking_id.clone() );
            }
            _ => {}
        }


        if relay.to != self.id()
        {
            let relay = relay.inc_hops();
            self.router.relay_wire(Wire::Relay(relay) );
        }
        else
        {
            // here we HANDLE the relay as this node is the recipient
            match &relay.payload
            {
                RelayPayload::Ping(id)=> {
                    let id = id.clone();
                    let reply = relay.reply(RelayPayload::Pong(id));
                    self.router.relay_wire(Wire::Relay(reply))?;
                }
                RelayPayload::Pong(id)=> {
                }
                RelayPayload::SearchResult(result)=>
                {
                    match &self.core
                    {
                        StarCore::Server(server) => {
                            let mut server = server.write().unwrap();
                            server.nearest_supervisors.insert(result.found, result.hops );
                        }
                        _ => {}
                    }
                }
                RelayPayload::RequestUniqueSeq => {
                    if self.core.is_central()
                    {
                        let reply = relay.reply(RelayPayload::ReportUniqueSeq(ReportUniqueSeqPayload{seq:self.seq().next().id}));
                        self.router.relay_wire(Wire::Relay(reply))?;
                    }
                    else {
                        self.error("RELAY PANIC! only central should be receiving ReportUniqueSeq from Relay")
                    }
                },
                RelayPayload::ReportSupervisorAvailable=> {
                    match &self.core
                    {

                        StarCore::Central(cluster) => {
                            let mut cluster = cluster.write()?;
                            cluster.available_supervisors.push(relay.from.clone());
                        }
                        _ => {
                            self.error("RELAY PANIC! only central should be receiving ReportSupervisorAvailable from Relay")
                        }
                    }
                }
                RelayPayload::RequestCreateSimulation(request_create_simulation)=> {
                    match &self.core
                    {
                        StarCore::Central(cluster) => {
                            let mut cluster = cluster.write()?;
                            let index = (cluster.available_supervisors_round_robin_index % cluster.available_supervisors.len());
                            let star = cluster.available_supervisors[index].clone();
                            let forward = Wire::Relay(Relay {
                                from: self.id(),
                                to: star,
                                payload: RelayPayload::AssignSimulationToSupervisor(ReportAssignSimulation {
                                    simulation: request_create_simulation.simulation.clone(),
                                    simulation_config: request_create_simulation.simulation_config.clone(),
                                    inform: relay.from
                                }),

                                transaction: None,
                                hops: 0
                            });
                            self.router.relay_wire(forward);
                        }
                        _ => {
                            self.error("RELAY PANIC! only central should be receiving RequestCreateSimulation from Relay")
                        }
                    }
                }
                RelayPayload::ReportUniqueSeq(seq_id)=> {
                    if relay.transaction.is_some()
                    {
                        let mut transaction = {
                            let mut connection_transactions = self.connection_transactions.lock()?;
                            connection_transactions.remove(&relay.transaction.unwrap())
                        };
                        if transaction.is_some()
                        {
                            let connection = transaction.unwrap();
                            connection.to_remote(Wire::ReportUniqueSeq(ReportUniqueSeqPayload { seq: seq_id.seq }));
                        } else {
                            self.error(format!("cannot find connection transaction {:?}", relay.transaction).as_str());
                        }
                    }
                }
                _ => { }
            }

        }


        Ok(())

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



impl WireListener for Star
{
    fn describe(&self)->String
    {
        format!("NodeKind: {}",self.core)
    }

    fn on_wire(&self, wire: Wire, mut connection: Arc<Connection>) -> Result<(), Error> {

println!("on_wire()  {} -> {}", wire, self.core,);
        match wire{
            Wire::ReportVersion(_)=> {
                // if we have a Node Id, return it
                if self.id.borrow().is_some()
                {
                    connection.to_remote(Wire::ReportNodeId(self.id()));
                }
                else
                {
                    connection.to_remote(Wire::RequestUniqueSeq);
                }
            },
            Wire::RequestUniqueSeq => {

                if self.core.is_central()
                {
                    connection.to_remote( Wire::ReportUniqueSeq(ReportUniqueSeqPayload{ seq: self.seq().next().id }));
                }
                else {
                    let transaction = self.seq().next();
                    let wire = Wire::Relay(
                            Relay{
                                from: self.id(),
                                to: Id::new(0, 0),
                                payload: RelayPayload::RequestUniqueSeq,
                                transaction: Option::Some(transaction.clone()),
                                hops: 0
                            }
                    );
                    {
                        let mut connection_transactions = self.connection_transactions.lock()?;
                        connection_transactions.insert(transaction, connection);
                    }

                    self.router.relay_wire(wire);
                }
            }
            Wire::ReportUniqueSeq(payload) => {
                self.init_with_sequence(payload.seq);
                connection.to_remote( Wire::ReportNodeId(self.id()));
                if connection.get_remote_node_id().is_some()
                {
                    self.router.add_external_connection(connection);
                    self.start();
                }
            }
            Wire::ReportNodeId(node_id) => {
                connection.add_found_node(node_id, NodeFind::new(1,u64::MAX));
                self.router.add_external_connection( connection);
                if self.is_init()
                {
                    self.start();
                }
            }
            Wire::NodeSearch(search) => {
                let mut search  = search.clone();
                search.hops = search.hops+1;

                // since the request came from this route we know that's where to find it
                connection.add_found_node(search.from.clone(), NodeFind::new(search.hops, self.timestamp() ));
                // we can also say the seeking node cannot be found in that direction
                connection.add_unfound_node(search.seeking_id.clone() );


                if search.seeking_id == self.id()
                {
                    connection.wire(Wire::Relay(
                        Relay{
                            from: self.id(),
                            to:  search.from.clone(),
                            payload: RelayPayload::NodeFound(search),
                            transaction: Option::Some(self.seq.borrow().as_ref().unwrap().next()),
                            hops: 0
                        }
                        ))?;
                }
                else if( search.hops < 0 )
                {
                    self.error("Dumping illegal payload hops < 0")
                }
                else if( search.hops > 16 )
                {
                    connection.wire(Wire::Relay(
                        Relay{
                            from: self.id(),
                            to:  search.from.clone(),
                            payload: RelayPayload::NodeNotFound(search),
                            transaction: Option::Some(self.seq.borrow().as_ref().unwrap().next()),
                            hops: 0
                        }
                    ))?;
                }
                else
                {
                    let mut search = search.clone();
                    self.router.relay_wire( Wire::NodeSearch(search) )?;
                }
            }
            Wire::NodeFound(search) => {
                connection.add_found_node(search.seeking_id.clone(), NodeFind::new(search.hops, self.timestamp() ));
            }
            Wire::MessageTransport(transport) => {
            }
            Wire::Relay(relay) => {
                self.on_relay(relay,connection);

            }
            Wire::Search(search) => {
                self.on_search(search,connection);

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

#[derive(Clone,Eq,PartialEq)]
pub enum StarKind
{
    Central,
    Server,
    Mesh,
    Gateway,
    Client,
    Supervisor
}

pub enum StarCore
{
    Central(RwLock<Cluster>),
    Server(RwLock<Server>),
    Mesh,
    Gateway,
    Client,
    Supervisor(RwLock<Supervisor>)
}

impl StarCore
{
    fn create_id(&self)->Option<Id>
    {
       match self{
           StarCore::Central(cluster) => Option::Some(Id::new(0, 0)),
           _ => Option::None
       }
    }

    fn is_central(&self)->bool
    {
        match self{
            StarCore::Central(_) => true,
            _ => false
        }
    }

    fn is_server(&self)->bool
    {
        match self{
            StarCore::Server(_) => true,
            _ => false
        }
    }


    pub fn kind(&self)->StarKind
    {
        match self
        {
            StarCore::Central(_) => StarKind::Central,
            StarCore::Server(_) => StarKind::Server,
            StarCore::Mesh => StarKind::Mesh,
            StarCore::Gateway => StarKind::Gateway,
            StarCore::Client => StarKind::Client,
            StarCore::Supervisor(_) => StarKind::Supervisor
        }
    }

}

pub struct Server
{
    pub nearest_supervisors: HashMap<Id,i32>
}

impl Server
{
    pub fn new()->Self
    {
        Server{
            nearest_supervisors: HashMap::new()
        }
    }
}

impl fmt::Display for StarCore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let r = match self {
            StarCore::Central(_) => {"Central"}
            StarCore::Server(_) => {"Server"}
            StarCore::Mesh => {"Mesh"}
            StarCore::Gateway => {"Gateway"}
            StarCore::Client => {"Client"}
            StarCore::Supervisor(_) => {"Supervisor"}
        };
        write!(f, "{}",r)
    }
}

pub struct Supervisor
{

}

impl Supervisor{
    pub fn new()->Self{
        Supervisor{}
    }
}

pub trait StarErrorHandler
{
    fn on_error(&self, message:&str);
}

pub struct PanicErrorHandler
{
}

impl PanicErrorHandler
{
    pub fn new()->Self
    {
        PanicErrorHandler{}
    }
}

impl StarErrorHandler for PanicErrorHandler
{
    fn on_error(&self, message: &str) {
        panic!("{}",message)
    }
}

pub struct LogErrorHandler
{
}

impl StarErrorHandler for LogErrorHandler
{
    fn on_error(&self, message: &str) {
        println!("{}",message);
    }
}

