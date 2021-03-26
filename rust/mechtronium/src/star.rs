use std::alloc::System;
use std::borrow::{Borrow, BorrowMut};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::RandomState;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock, Weak};

use wasmer::{Cranelift, JIT, Module, Store};

use mechtron_common::artifact::Artifact;
use mechtron_common::configs::{Configs, Keeper, NucleusConfig, Parser, SimConfig};
use mechtron_common::core::*;
use mechtron_common::id::{Id, IdSeq, MechtronKey};
use mechtron_common::message::{Cycle, DeliveryMoment, MechtronLayer, Message, MessageKind, MessageTransport, To};

use crate::artifact::MechtroniumArtifactRepository;
use crate::cache::{Cache, default_cache};
use crate::cluster::Cluster;
use crate::error::Error;
use crate::mechtron::CreatePayloadsBuilder;
use crate::nucleus::{Nuclei, NucleiContainer, Nucleus, NucleusBomb};
use crate::router::{HasNucleus, InternalRouter, LocalRouter, NetworkRouter, SharedRouter};
use crate::simulation::Simulation;
use crate::transport::{AssignNucleus, Connection, ExternalRoute, NodeFind, NodeRouter, Relay, RelayPayload, ReportAssignSimulation, ReportAssignSimulationToServer, ReportSupervisorForSim, ReportUniqueSeqPayload, ReqCreateSim, Route, Search, SearchKind, SearchResult, Unwind, UnwindPayload, Wire, WireListener, ReportNucleusReady, ReportNucleusNodePayload, NucleusReadyListener, WatchResult, WatchResultKind, Broadcast, Watch};
use crate::transport::RelayPayload::{ReportSupervisorAvailable, RequestCreateSimulation, ReportNucleusNode};

static MAX_HOPS : i32 = 16;

pub struct Star {
    pub id: RefCell<Option<Id>>,
    pub seq: RefCell<Option<Arc<IdSeq>>>,
    pub local: RefCell<Option<Arc<Local>>>,
    pub core: StarCore,
    pub cache: Arc<Cache>,
    pub router: Arc<NodeRouter>,
    connection_transactions: Mutex<HashMap<Id,Arc<Connection>>>,
    pub error_handler: RefCell<Box<dyn StarErrorHandler>>,
    pub nearest_supervisors: Mutex<HashMap<Id,i32>>,
    pub transaction_watchers: Mutex<HashMap<Id,Weak<dyn TransactionWatcher>>>,
    pub debug: bool,
    pub searches: Mutex<HashMap<Id,Arc<dyn TransactionWatcher>>>
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
            error_handler: RefCell::new( Box::new(LogErrorHandler{}) ),
            nearest_supervisors: Mutex::new(HashMap::new() ),
            transaction_watchers: Mutex::new( HashMap::new() ),
            debug: true,
            searches: Mutex::new(HashMap::new() )
        };

        if rtn.core.is_central()
        {
           rtn.init_with_sequence(0);
        }

        rtn
    }

    pub fn request_simulation_supervisor(&self, sim_id: Id, transaction_watcher: Arc<dyn TransactionWatcher> )->Result<(),Error>
    {
        let payload = RelayPayload::RequestSupervisorForSim(sim_id);
        let mut relay = Relay::to_central(self.id(), payload );
        relay.transaction = Option::Some(self.seq().next());

        {
            self.transaction_watchers.lock()?.insert(relay.transaction.unwrap().clone(), Arc::downgrade(&transaction_watcher));
        }

        let wire = Wire::Relay(relay);
        self.router.relay_wire(wire);
        Ok(())
    }

    pub fn request_nucleus_star(&self, supervisor: Id, nucleus: Id, transaction_watcher: Arc<dyn TransactionWatcher>) -> Result<(), Error>
    {
        let payload = RelayPayload::RequestNucleusNode(nucleus);
        let mut relay = Relay::new(self.id(), supervisor, payload);
        relay.transaction = Option::Some(self.seq().next());

        {
            self.transaction_watchers.lock()?.insert(relay.transaction.unwrap().clone(), Arc::downgrade(&transaction_watcher));
        }

        let wire = Wire::Relay(relay);
        self.router.relay_wire(wire);
        Ok(())
    }


    pub fn request_create_simulation(&self, simulation_config: Artifact, transaction_watcher: Arc<dyn TransactionWatcher>) -> Result<(), Error>
    {
        let nearest_supervisors: HashSet<Id> =
            {
                let nearest_supervisors = self.nearest_supervisors.lock()?;
                nearest_supervisors.keys().map(|id| id.clone()).collect()
            };

        let request = ReqCreateSim
        {
            simulation: self.seq().next(),
            simulation_config: simulation_config,
            nearby_supervisors: nearest_supervisors,
        };

        let relay = RelayPayload::RequestCreateSimulation(request);
        let relay = Relay{
            from: self.id(),
            to: Id::new(0,0),
            payload: relay,
            transaction: Option::Some(self.seq().next()),
            inform: Option::Some(self.id()),
            hops: 0
        };


        {
            self.transaction_watchers.lock()?.insert(relay.transaction.unwrap().clone(), Arc::downgrade(&transaction_watcher));
        }

        let wire = Wire::Relay(relay);
        self.router.relay_wire(wire);
        Ok(())
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

        if let StarCore::Ext(ext) = &self.core
        {
            ext.on_init(self);
        }

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
                let search = Search::new(self.id(), SearchKind::StarKind(StarKind::Supervisor), self.seq().next() );
                self.router.relay_wire(Wire::Search(search));
            },
            StarCore::Client => {
                let search = Search::new(self.id(), SearchKind::StarKind(StarKind::Supervisor),self.seq().next() );
                self.router.relay_wire(Wire::Search(search));
            },
            _ => {}
        }
    }


    pub fn shutdown(&self) {}

    pub fn create_sim_from_scratch(&self, config: Arc<SimConfig>, sim_id: Id) -> Result<Id, Error> {

        if self.local.borrow().is_none()
        {
            return Err("local is none".into())
        }

        let id = self.local.borrow().as_ref().unwrap().sources.create_sim(config, sim_id )?;

       Ok(id)
    }


    pub fn create_nucleus(&self, config: Arc<NucleusConfig>, sim: Id) -> Result<Id, Error> {

        if self.local.borrow().is_none()
        {
            return Err("local is none".into())
        }

        let id = self.local.borrow().as_ref().unwrap().sources.create( sim, Option::None, config )?;

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

        if search.max_hops > MAX_HOPS
        {
            self.error(format!("Illegal max search hops: {}",search.max_hops).as_str());
            return Err(format!("max hops cannot be more than {}",MAX_HOPS).into());
        }
        else if search.hops.len() > search.max_hops as usize
        {
            self.error("Too many search hops!");
            return Err(format!("too many search hops {}",search.hops.len()).into());
        }

        if search.kind.matches(self)
        {
            let unwind = Unwind::new(self.id(), UnwindPayload::SearchFoundResult(SearchResult{
                sought: Option::Some(self.id()),
                kind: search.kind.clone(),
                transactions: search.transactions.clone()
            }), search.hops.clone(), search.hops.len() as _ );

            self.router.relay_wire(Wire::Unwind(unwind) )?;
            if search.kind.is_multiple_match()
            {
                self.branch_search(search, connection);
            }
        }
        else {
            self.branch_search(search,connection);
        }

        Ok(())
    }

    fn branch_search( &self, search: Search, connection: Arc<Connection> )
    {

        // create a new search transaction
        let transaction = self.seq().next();
        let search = search.push(self.id(),transaction );
        {
            let mut searches = self.searches.lock().unwrap();
            searches.insert(transaction, Arc::new(SearchTransactionWatcher { searches: Cell::new(0), connection: connection.clone() }));
        };
        self.router.relay_wire_excluding( Wire::Search(search), connection.get_remote_node_id() );
    }


    fn on_broadcast( &self, broadcast: Broadcast, connection: Arc<Connection> )->Result<(),Error>
    {
        self.router.relay_wire(Wire::Broadcast(broadcast) );
        Ok(())
    }

    fn on_unwind( &self, unwind: Unwind, connection: Arc<Connection> )->Result<(),Error>
    {

        if unwind.path.len() > MAX_HOPS as usize
        {
            self.error("Too many payload hops!");
            return Err("Too many payload Hops!".into());
        }

        // handled the same no matter if this is the To node or not
        match unwind.payload
        {
            UnwindPayload::SearchFoundResult(ref result) => {

                match result.kind
                {
                    SearchKind::StarKind(ref kind) => {
                        match kind
                        {
                            StarKind::Supervisor => {
                                if result.sought.is_some()
                                {
                                    let mut nearest_supervisors = self.nearest_supervisors.lock()?;
                                    nearest_supervisors.insert(result.sought.unwrap().clone(), unwind.hops.clone());
                                }
                                else {
                                    println!("expected: search FOUND");
                                 }
                            }
                            _=>{}
                        }
                    }
                    SearchKind::StarId(id) => {
                        connection.add_found_node(id, NodeFind::new(unwind.hops.clone(), self.timestamp()));
                        self.router.notify_found( &unwind , connection.clone() );
                    }
                }

                match &self.core
                {
                    StarCore::Server(server) => {
                        match &result.kind
                        {
                            SearchKind::StarKind(ref kind) => {

                                match kind{
                                    StarKind::Supervisor =>  {
                                        let mut server = server.write().unwrap();
                                        if( server.supervisor.is_none() )
                                        {
                                            server.supervisor = Option::Some(result.sought.unwrap());
                                            let relay = Relay::new(self.id(), server.supervisor.unwrap().clone(), RelayPayload::PledgeServices );
                                            self.router.relay_wire(Wire::Relay(relay));
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
                        };


                    }
                    _ => {}
                }
            }
            UnwindPayload::SearchNotFoundResult(ref search) => {
                match search.kind{
                    SearchKind::StarKind(_) => {}
                    SearchKind::StarId(id) => {
                        connection.add_unfound_node(id);
                    }
                }
// CHECK HERE FOR UNFOUND TRANSACTION
            }
        }

        let unwind = unwind.pop();
        if !unwind.path.is_empty()
        {
            self.router.relay_wire(Wire::Unwind(unwind));
        }

        Ok(())
    }

    fn on_relay( &self, relay: Relay, connection: Arc<Connection> )->Result<(),Error>
    {

        if relay.hops < 0
        {
            self.error("Illegal payload hops!");
            return Err("Illegal payload hops!".into());
        }
        else if relay.hops > MAX_HOPS
        {
            self.error("Too many payload hops!");
            return Err("Too many payload hops!".into());
        }

        if relay.to != self.id()
        {

            if let RelayPayload::Watch(watch) = &relay.payload
            {
                connection.add_watch(relay.from.clone(), watch.clone());
            }
            else if let RelayPayload::UnWatch(watch) = &relay.payload
            {
                connection.remove_watch(relay.from.clone(), watch);
            }

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

                    if relay.transaction.is_none()
                    {
                        self.error("RequestCreateSimulation should have a transaction");
                    }
                    if relay.inform.is_none()
                    {
                        self.error("RequestCreateSimulation should have an inform");
                    }
                    match &self.core
                    {
                        StarCore::Central(cluster) => {

                            let (sim_id, star) =
                            {
                                let sim_id = self.seq().next();
                                let mut cluster = cluster.write()?;
                                let index = (cluster.available_supervisors_round_robin_index % cluster.available_supervisors.len());
                                let star = cluster.available_supervisors[index].clone();
                                cluster.simulation_supervisor_to_star.insert( sim_id.clone(), star.clone() );
                                (sim_id,star)
                            };

                            let forward = Wire::Relay(Relay {
                                from: self.id(),
                                to: star,
                                payload: RelayPayload::AssignSimulationToSupervisor(ReportAssignSimulation {
                                    simulation_id: sim_id.clone(),
                                    simulation_config: request_create_simulation.simulation_config.clone(),
                                }),

                                inform: relay.inform.clone(),
                                transaction: relay.transaction.clone(),
                                hops: 0
                            });
                            self.router.relay_wire(forward);
                        }
                        _ => {
                            self.error("RELAY PANIC! only central should be receiving RequestCreateSimulation from Relay")
                        }
                    }
                }
                RelayPayload::RequestSupervisorForSim(sim_id) if self.core.is_central() =>{
println!("RECEIVED REQUEST SUPERVISOR FOR SIM");
                    match &self.core{
                        StarCore::Central(cluster) => {
                            let cluster = cluster.read()?;
                            let supervisor = cluster.simulation_supervisor_to_star.get(sim_id );

                            if supervisor.is_some()
                            {
                                let report = ReportSupervisorForSim {
                                    simulation: sim_id.clone(),
                                    star: supervisor.unwrap().clone()
                                };
                                println!("RECEIVED REQUEST SUPERVISOR FOR SIM.... RELAY");
                                self.router.relay_wire(Wire::Relay(relay.reply(RelayPayload::ReportSupervisorForSim(report))));
                            } else {
                                self.error(format!("RELAY PANIC! cannot find sim id: {:?}", sim_id).as_str())
                            }
                        }
                        _ => {}
                    }
                }
                RelayPayload::RequestNucleusNode(nucleus) if self.core.is_supervisor() => {
                    unimplemented!()
                }
                RelayPayload::AssignSimulationToSupervisor(assign_simulation) =>
                    {
                        if relay.inform.is_none() || relay.transaction.is_none()
                        {
                            self.error("AssignSimulationToSupervistor should have an inform");
                        }
                        match &self.core {
                            StarCore::Supervisor(supervisor) => {
                                let supervisor = supervisor.write()?;
                                match supervisor.select_server()
                           {
                               Ok(server_id) => {
                                   let wire = Wire::Relay(Relay{
                                       from: self.id(),
                                       to:  server_id,
                                       payload: RelayPayload::AssignSimulationToServer(ReportAssignSimulationToServer{
                                           simulation_id: assign_simulation.simulation_id.clone(),
                                           simulation_config: assign_simulation.simulation_config.clone(),
                                       }),
                                       inform: relay.inform.clone(),
                                       transaction: relay.transaction.clone(),
                                       hops: 0
                                   });
                                   self.router.relay_wire(wire);
                               }
                               Err(err) => {
                                   self.relay_error("supervisor has no available servers", relay.inform.clone(), relay.transaction.clone());
                               }
                           }
                            }
                            _ => {
                                self.error("RELAY PANIC! attempt to assign a simulation to a non Supervisor!");
                            }
                        }
                    }
                RelayPayload::RequestNucleus(request) =>
                    {
                        if let StarCore::Supervisor(supervisor) = &self.core {
                            let supervisor = supervisor.write()?;
                            if let Ok(server_id) = supervisor.select_server()
                            {
                                let mut listeners = request.listeners.clone();
                                listeners.push( NucleusReadyListener{
                                    star: self.id(),
                                    transaction: self.seq().next() // transaction doesn't matter in this case
                                });
                                let wire = Wire::Relay(Relay {
                                    from: self.id(),
                                    to: server_id,
                                    payload: RelayPayload::AssignNucleus(AssignNucleus {
                                        sim: request.sim.clone(),
                                        config: request.config.clone(),
                                        listeners: listeners
                                    }),
                                    inform: relay.inform.clone(),
                                    transaction: relay.transaction.clone(),
                                    hops: 0,
                                });
                                self.router.relay_wire(wire);
                            } else {
                                self.relay_error("supervisor has no available servers", relay.inform.clone(), relay.transaction.clone());
                            }
                        }
                    }
                RelayPayload::AssignNucleus(assign) =>
                    {
                        if let StarCore::Server(server) = &self.core {
                            //let server = server.write()?;
                            let config = self.cache.configs.nucleus.get( &assign.config )?;
                            let nucleus = self.create_nucleus(config,assign.sim.clone())?;
                            for listener in &assign.listeners
                            {
                                let wire = Wire::Relay(Relay {
                                    from: self.id(),
                                    to: listener.star.clone(),
                                    payload: RelayPayload::ReportNucleusReady(ReportNucleusReady{
                                        sim: assign.sim.clone(),
                                        nucleus: nucleus,
                                        star: self.id()
                                    }),
                                    inform: relay.inform.clone(),
                                    transaction: Option::Some(listener.transaction.clone()),
                                    hops: 0,
                                });
                                self.router.relay_wire(wire);
                            }
                        }
                        else{
                            self.relay_error("can only AssignNucleus to a server", relay.inform.clone(), relay.transaction.clone());
                        }
                    }
                RelayPayload::ReportNucleusReady(ready) =>
                {
                   if let StarCore::Supervisor(supervisor) = &self.core
                   {
                       let mut supervisor = supervisor.write()?;
                       supervisor.nucleus_to_star.insert(ready.nucleus.clone(), ready.star );
                       supervisor.nucleus_to_sim.insert(ready.nucleus, ready.sim );
                   }
                   self.process_transaction_notification_final(&relay);
                }
                RelayPayload::RequestNucleusNode(nucleus) =>
                    {
                        if let StarCore::Supervisor(supervisor) = &self.core
                        {
                            let mut supervisor = supervisor.read()?;
                            let node = supervisor.nucleus_to_star.get(nucleus);
                            let sim = supervisor.nucleus_to_sim.get(nucleus);
                            if node.is_some() && sim.is_some()
                            {
                                let relay = relay.reply(ReportNucleusNode(ReportNucleusNodePayload { sim: sim.unwrap().clone(), node: node.unwrap().clone() }));
                                self.router.relay_wire(Wire::Relay(relay));
                            }
                            else {

                                self.relay_error(format!("could not find nucleus {:?}",nucleus).as_str(), relay.inform.clone(), relay.transaction.clone());
                            }
                        }

                    }

                RelayPayload::ReportNucleusDetached(nucleus) =>
                    {
                        if let StarCore::Supervisor(supervisor) = &self.core
                        {
                            let mut supervisor = supervisor.write()?;
                            supervisor.nucleus_to_star.remove(&nucleus );
                        }
                    }
                RelayPayload::AssignSimulationToServer(report) =>

                    {
                        match &self.core {
                            StarCore::Server(_) => {
                                match self.cache.configs.sims.cache(&report.simulation_config)
                                {
                                    Ok(_) => {
                                        match self.cache.configs.sims.get(&report.simulation_config)
                                        {
                                        Ok(sim_config) => {
                                           match self.create_sim_from_scratch(sim_config, report.simulation_id)
                                           {
                                               Ok(simulation_id) => {
                                                   if( relay.inform.is_some() )
                                                   {
                                                       let wire = Wire::Relay(Relay {
                                                           from: self.id().clone(),
                                                           to: relay.inform.unwrap(),
                                                           payload: RelayPayload::NotifySimulationReady(simulation_id),
                                                           inform: None,
                                                           transaction:  relay.transaction.clone(),
                                                           hops: 0
                                                       });
                                                       self.router.relay_wire(wire);
                                                   }
                                                   else {
                                                       self.error_handler.borrow().on_error("assign sim should have an INFORM");
                                                   }
                                               }
                                               Err(err) => {
                                                   self.error_handler.borrow().on_error(format!("{:?}",err).as_str());
                                               }
                                           }
                                        }
                                        Err(_) => {
                                            self.error_handler.borrow().on_error("bad simulation config");
                                        }
                                    }
                                }
                                Err(_) => {
                                    self.error_handler.borrow().on_error("bad simulation config");
                                }
                            }
                        }
                        _ => self.error("RELAY PANIC! attempt to assign a simulation to a non Server!")
                    }

                },
                RelayPayload::NotifySimulationReady(simulation_id)=>
                {
                    self.process_transaction_notification_final( &relay );
                }
                RelayPayload::ReportSupervisorForSim(report)=>
                 {
                        self.process_transaction_notification_final( &relay );
                 }
                RelayPayload::ReportNucleusNode(report)=>
                {
                       self.process_transaction_notification_final( &relay );
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
                },
                RelayPayload::PledgeServices =>
                {
                    match &self.core {
                        StarCore::Supervisor(supervisor) => {
                           let mut supervisor = supervisor.write()?;
                           supervisor.servers.push(relay.from.clone() );
                        }
                        _ => {}
                    }
                }
                RelayPayload::Watch(watch)=>
                {
                    match &watch
                    {
                        Watch::Nucleus(nucleus_id) => {
                            if self.local().sources.has_nucleus(nucleus_id)
                            {
                               let bomb = self.local().sources.create_bomb(nucleus_id)?;
                               let reply = relay.reply( RelayPayload::WatchResult(WatchResult{
                                   kind: WatchResultKind::Nucleus(bomb)
                               }));
                               self.router.relay_wire(Wire::Relay(reply));
                            }
                            else {
                                self.error_handler.borrow().on_error(format!("this star does not contain nucleus {:?}",nucleus_id).as_str());
                            }
                        }
                    }
                }
                RelayPayload::WatchResult(result)=>
                    {
                        match &result.kind
                        {
                            WatchResultKind::Nucleus(bomb) => {
unimplemented!()
//                                self.local().replicas.update_nucleus(bomb.clone());
                            }
                        }
                    }
                _ => { }
            }

        }


        Ok(())

    }

    fn relay_error( &self, message: &str, to: Option<Id>, transaction: Option<Id> )
    {
        // right now doesn't do anything

    }

    fn process_transaction_notification(&self, relay: &Relay )
    {
        if( relay.transaction.is_some() )
        {
            let watcher: Option<Arc<dyn TransactionWatcher>>= {
                let mut transaction_watchers = self.transaction_watchers.lock();
                match transaction_watchers{
                    Ok(transaction_watchers) => {
                        let watcher = transaction_watchers.get(&relay.transaction.unwrap()  );
                        match watcher
                        {
                            None => {

println!("NO WATCHERS");
                                Option::None
                            }
                            Some(watcher) => {
println!("sOME WATCHERS");
                                match watcher.upgrade(){
                                    None => {Option::None}
                                    Some(watcher) => {
                                        Option::Some(watcher.clone())
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => {
                        Option::None
                    }
                }
            };

            match watcher{
                None => {}
                Some(watcher) => {
                  watcher.on_transaction(&Wire::Relay(relay.clone()), self );
                }
            }
        }
    }

    fn process_transaction_notification_final(&self, relay: &Relay )
    {
        if( relay.transaction.is_some() )
        {
            let watcher: Option<Arc<dyn TransactionWatcher>>= {
                let mut transaction_watchers = self.transaction_watchers.lock();
                match transaction_watchers{
                    Ok(mut transaction_watchers) => {
                        let watcher = transaction_watchers.remove(&relay.transaction.unwrap()  );

                        match watcher
                        {
                            None => {
println!("REMOVED NONE");
                                Option::None
                            }
                            Some(watcher) => {
println!("REMOVED SOME");
                                match watcher.upgrade(){
                                    None => {Option::None}
                                    Some(watcher) => {
                                        Option::Some(watcher.clone())
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => {
                        Option::None
                    }
                }
            };

            match watcher{
                None => {}
                Some(watcher) => {
                    watcher.on_transaction(&Wire::Relay(relay.clone()), self );
                }
            }
        }
    }



}


pub struct Local {
    sources: Arc<Nuclei>,
    replicas: Replicas,
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
            cache: cache.clone(),
            replicas: Replicas::new()
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

/*
println!("on_wire({})  {} -> {}", match &wire{

    Wire::Relay(relay) => {

        let rtn = format!("<{:?}->{:?}>{}",relay.from,relay.to,relay.payload);
        rtn.clone()
    }
    _ => "".to_string()
}, wire, self.core,);

 */
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
                                inform: Option::None,
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
            Wire::MessageTransport(transport) => {
            }
            Wire::Relay(relay) => {
                self.on_relay(relay,connection);
            }
            Wire::Search(search) => {
                self.on_search(search,connection);
            }
            Wire::Unwind(unwind) => {
                self.on_unwind(unwind,connection);
            }
            Wire::Broadcast(broadcast) => {
                self.on_broadcast(broadcast,connection);
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
    Supervisor,
    Ext
}

pub enum StarCore
{
    Central(RwLock<Cluster>),
    Server(RwLock<Server>),
    Mesh,
    Gateway,
    Client,
    Supervisor(RwLock<Supervisor>),
    Ext(Box<dyn StarExtension>)
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

    fn is_server(&self) -> bool
    {
        match self {
            StarCore::Server(_) => true,
            _ => false
        }
    }

    fn is_supervisor(&self) -> bool
    {
        match self {
            StarCore::Supervisor(_) => true,
            _ => false
        }
    }


    pub fn kind(&self) -> StarKind
    {
        match self
        {
            StarCore::Central(_) => StarKind::Central,
            StarCore::Server(_) => StarKind::Server,
            StarCore::Mesh => StarKind::Mesh,
            StarCore::Gateway => StarKind::Gateway,
            StarCore::Client => StarKind::Client,
            StarCore::Supervisor(_) => StarKind::Supervisor,
            StarCore::Ext(_) => StarKind::Ext,
        }
    }

}

pub struct Server
{
    pub nearest_supervisors: HashMap<Id,i32>,
    pub supervisor: Option<Id>
}

impl Server
{
    pub fn new()->Self
    {
        Server{
            nearest_supervisors: HashMap::new(),
            supervisor: Option::None
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
            StarCore::Ext(_) => {"Ext"}
        };
        write!(f, "{}",r)
    }
}

pub struct Supervisor
{
    pub servers: Vec<Id>,
    pub nucleus_to_star: HashMap<Id, Id>,
    pub nucleus_to_sim: HashMap<Id, Id>,
}

impl Supervisor{
    pub fn new()->Self{
        Supervisor {
            servers: vec!(),
            nucleus_to_star: HashMap::new(),
            nucleus_to_sim: HashMap::new()
        }
    }

    pub fn select_server(&self)->Result<Id,Error>
    {
        if self.servers.is_empty()
        {
            return Err("no servers available".into());
        }

        // right now we just grab the first because this algorithm isn't very sophisticated

        Ok(self.servers[0].clone())
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


pub trait TransactionWatcher
{
    fn on_transaction( &self, wire: &Wire, star: &Star) -> TransactionResult;
}


pub struct SearchTransactionWatcher{
    pub searches: Cell<i32>,
    pub connection: Arc<Connection>
}

pub enum TransactionResult
{
    Inconclusive,
    Final
}

impl TransactionWatcher for SearchTransactionWatcher
{
    fn on_transaction(&self, wire: &Wire, star: &Star)-> TransactionResult{
        self.searches.replace(self.searches.get()-1);

        match wire{
            Wire::Relay(relay ) => {

                match &relay.payload {
                    RelayPayload::SearchNotFound(search) => {
                        let mut search = search.clone();
                        search.pop();
                        if self.searches.get() <= 0 && !search.transactions.is_empty()
                        {
                            self.connection.to_remote( Wire::Relay( Relay{
                                from: star.id(),
                                to: self.connection.get_remote_node_id().unwrap(),
                                payload: RelayPayload::SearchNotFound(search.clone()),
                                inform: None,
                                transaction: Option::Some(search.transactions.last().unwrap().clone()),
                                hops: 0
                            } ) );
                        }
                    }
                    _ => {}
               }
            }
            Wire::Unwind(unwind ) => {}
            _ => {
            }
        }

        TransactionResult::Final

    }
}


pub struct Replicas
{
    pub watching: HashSet<Id>,
    pub nucleus_to_bomb: HashMap<Id,NucleusBomb>
}

impl Replicas
{
   pub fn new()->Self
   {
       Replicas{
           watching: HashSet::new(),
           nucleus_to_bomb: HashMap::new()
       }
   }

   pub fn watch( &mut self, nucleus: &Id )
   {
       self.watching.insert(nucleus.clone() );
   }

  pub fn unwatch( &mut self, nucleus: &Id )
  {
      self.nucleus_to_bomb.remove(nucleus);
      self.watching.remove(nucleus);
  }

  pub fn update_nucleus( &mut self, bomb: NucleusBomb )
  {
      if self.watching.contains(&bomb.nucleus )
      {
          self.nucleus_to_bomb.insert(bomb.nucleus.clone(), bomb );
      }
  }
}



pub trait StarExtension
{
    fn on_init( &self, star: &Star );
}
