use std::{fmt, thread};
use std::borrow::Borrow;
use std::cell::{Cell, RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::{Arc, Mutex, RwLock, Weak};

use mechtron_common::artifact::Artifact;
use mechtron_common::id::{Id, IdSeq};
use mechtron_common::message::{Message, MessageTransport};

use crate::error::Error;
use crate::transport::RouteProblem::{NucleusNotKnown, StarUnknown};
use crate::router::NetworkRouter;
use crate::star::{Star, StarCore, StarKind};
use std::collections::hash_map::RandomState;
use crate::nucleus::NucleusBomb;

static PROTOCOL_VERSION: i32 = 100_001;


pub fn connect( a: Arc<dyn WireListener>, b: Arc<dyn WireListener> )->(Arc<Connection>,Arc<Connection>)
{
    let mut a = Arc::new(Connection::new(a));
    let mut b= Arc::new(Connection::new(b));
    a.remote.replace(Option::Some( b.clone() ));
    b.remote.replace(Option::Some( a.clone() ));

    a.this.replace(Option::Some( Arc::downgrade(&a.clone() )));
    b.this.replace(Option::Some( Arc::downgrade(&b.clone() )));

    a.init();
    b.init();

    a.unblock();
    b.unblock();

    (a,b)
}

#[derive(Clone,PartialEq,Eq)]
pub enum ConnectionStatus{
    WaitVersion,
    WaitNodeId,
    Ready,
    Error
}

pub struct Connection
{
    status: RefCell<ConnectionStatus>,
    local: Arc<dyn WireListener>,
    remote: RefCell<Option<Arc<Connection>>>,
    remote_node_id: RefCell<Option<Id>>,
    this: RefCell<Option<Weak<Connection>>>,
    unique_requested: Cell<bool>,
    remote_seq_id: RefCell<Option<i64>>,
    found_nodes: RefCell<HashMap<Id,NodeFind>>,
    unfound_nodes: RefCell<HashSet<Id>>,
    queue: RefCell<Vec<Wire>>,
    block: Cell<bool>,
    watches: RefCell<HashSet<Watch>>
}

impl Connection
{
    pub fn init(&self)
    {
        self.to_remote( Wire::ReportVersion(PROTOCOL_VERSION) );
    }


    pub fn is_error(&self)->bool
    {
        *self.status.borrow() == ConnectionStatus::Error
    }

    pub fn is_ok(&self)->bool
    {
        return !self.is_error();
    }

    pub fn get_remote_node_id(&self)->Option<Id>
    {
        self.remote_node_id.borrow().clone()
    }

    pub fn new( local: Arc<dyn WireListener>)->Self
    {
        Connection{
            status: RefCell::new(ConnectionStatus::WaitVersion),
            local: local,
            remote: RefCell::new(Option::None),
            this: RefCell::new(Option::None),
            remote_node_id: RefCell::new(None),
            unique_requested: Cell::new(false),
            remote_seq_id:RefCell::new(Option::None),
            found_nodes: RefCell::new(HashMap::new()),
            unfound_nodes: RefCell::new(HashSet::new()),
            queue: RefCell::new(vec!()),
            block: Cell::new(true),
            watches: RefCell::new(HashSet::new())
        }
    }

    pub fn add_watch( &self, star: Id, watch: Watch )
    {
        let mut watches = self.watches.borrow_mut();
        watches.insert(watch);
    }

    pub fn remove_watch( &self, star: Id, watch: &Watch )
    {
        let mut watches = self.watches.borrow_mut();
        watches.remove(watch);
    }


    pub fn to_remote(&self, wire: Wire) ->Result<(),Error>
    {
let remote = self.remote.borrow().as_ref().unwrap().local.clone();
//println!("to_remote() {} - {} -> {}", self.local.describe(), wire, remote.describe());
        match &wire{
            Wire::ReportUniqueSeq(seq) => {
                self.remote_seq_id.replace( Option::Some(seq.seq.clone()) );
            }
            _ => {}
        }
        let remote = self.remote.borrow();
        remote.as_ref().unwrap().receive( wire);
        Ok(())
    }

    fn error( &self, message: &str )
    {
        println!("{}",message);
        self.status.replace(ConnectionStatus::Error);
    }

    pub fn add_found_node(&self, node: Id, find: NodeFind )
    {
        self.found_nodes.borrow_mut().insert(node,find);
    }

    // remove any that are older that 'since'
    pub fn sweep(&self, since: u64 )
    {
        let mut rm = vec!();
        let found_nodes = self.found_nodes.borrow();
        for (id,find) in found_nodes.iter()
        {
            if find.timestamp < since
            {
                rm.push(id);
            }
        }
        for id in rm
        {
            self.found_nodes.borrow_mut().remove(&id);
        }
    }

    pub fn add_unfound_node( &self, node_id: Id )
    {
        //println!("unfound: {:?} for {:?}", node_id, self.remote_node_id);
        self.unfound_nodes.borrow_mut().insert(node_id);
    }

    fn this(&self)->Arc<Connection>
    {
        self.this.borrow().as_ref().unwrap().upgrade().unwrap().clone()
    }

    fn unblock(&self)
    {
        self.set_block(false);
    }

    fn set_block(&self, block: bool )
    {
        self.block.replace(block);

        if !self.block.get()
        {
            self.flush();
        }
    }

    fn receive(&self, wire: Wire)
    {
        self.queue.borrow_mut().push(wire );
        if !self.block.get()
        {
            self.flush();
        }
    }

    fn flush(&self)
    {
        let mut wires = vec!();
        for wire in self.queue.borrow_mut().drain(..)
        {
            wires.push(wire);
        }

        for wire in wires
        {
            let status = { (*self.status.borrow()).clone() };
            match status
            {
                ConnectionStatus::WaitVersion => {
                    match wire {
                        Wire::ReportVersion(version) => {
                            if version != PROTOCOL_VERSION
                            {
                                self.error("connection ERROR. did not report the expected VERSION")
                            } else {
                                self.status.replace(ConnectionStatus::WaitNodeId);
                                self.local.on_wire(wire, self.this());
                            }
                        }
                        _ => {
                            self.error(format!("connection ERROR. expected Version. Got {}", wire).as_str());
                        }
                    }
                }
                ConnectionStatus::WaitNodeId => match &wire {
                    Wire::ReportNodeId(remote_node_id) => {
                        if self.remote_seq_id.borrow().is_some() && self.remote_seq_id.borrow().unwrap() != remote_node_id.seq_id
                        {
                            self.error("cannot report a node id that differs from a unique sequence that has already been provided")
                        } else {
                            self.status.replace(ConnectionStatus::Ready);
                            self.remote_node_id.replace(Option::Some(remote_node_id.clone()));
                            self.local.on_wire(wire, self.this());
                        }
                    },
                    Wire::RequestUniqueSeq =>
                        {
                            if self.unique_requested.get()
                            {
                                self.error("cannot request uniques more than once");
                            } else {
                                self.unique_requested.replace(true);
                                self.local.on_wire(wire, self.this());
                            }
                        }
                    Wire::ReportUniqueSeq(seq) =>
                        {
                            self.local.on_wire(wire, self.this());
                        },

                    _ => {
                        self.error(format!("connection ERROR. expected ReportNodeId. Got {} for {}", wire, self.local.describe()).as_str());
                    }
                },

                ConnectionStatus::Ready => {
                    match wire {
                        Wire::ReportVersion(_) => {
                            self.error(format!("version should have already been reported").as_str());
                        }
                        Wire::ReportNodeId(_) => {
                            self.error(format!("cannot report node id more than once").as_str());
                        }
                        Wire::RequestUniqueSeq => {
                            self.error(format!("cannot request unique seq after node id has been set").as_str());
                        }
                        wire => {
                            self.local.on_wire(wire, self.this());
                        }
                    }
                }

                ConnectionStatus::Error => {
                    println!("Connection is Errored, no further processing.");
                    return;
                }
            }
//        let connection = self.this.borrow().as_ref().unwrap().upgrade().unwrap().clone();
//        self.local.on_wire(wire,connection);
        }
    }

    pub fn panic( &self, message: Message )
    {

    }

    pub fn close(&self)
    {

    }
}

pub struct NodeFind
{
    pub timestamp: u64,
    pub hops: i32
}

impl NodeFind
{
    pub fn new(hops:i32,timestamp:u64)->Self
    {
        NodeFind{
            hops: hops,
            timestamp: timestamp
        }
    }
}

pub struct InternalRouter
{
    routes: RwLock<Vec<Box<dyn InternalRoute>>>,
}

impl InternalRouter
{
    pub fn new()->Self
    {
        InternalRouter{
            routes: RwLock::new(vec!()),

        }
    }

    pub fn has_nucleus( &self, nucleus_id: Id )->bool
    {
        let routes = self.routes.read().unwrap();
        for route in &(*routes)
        {
            if route.has_nucleus(&nucleus_id)
            {
                return true;
            }
        }

        false
    }


    pub fn add_route( &mut self, route: Box<dyn InternalRoute> )
    {
        let mut routes = self.routes.write().expect("hopefully we haven't poisoned the lock");
        routes.push( route );
    }

    pub fn relay( &self, message_transport: MessageTransport )->Result<(),Error>
    {
        let routes = self.routes.read().unwrap();
        for route in &(*routes)
        {
            if route.has_nucleus(&message_transport.message.to.tron.nucleus)
            {
unimplemented!();
//                route.relay(message_transport);
                break;
            }
        }

        Err("could not find route".into())
    }


}

pub struct ExternalRouter
{
    inner: RwLock<ExternalRouterInner>,
    hold: Mutex<Vec<(Wire, Option<Id>)>>,
    star: RefCell<Option<Id>>,
    seq: IdSeq,
}

impl ExternalRouter
{
    pub fn new() -> Self
    {
        ExternalRouter {
            inner: RwLock::new(ExternalRouterInner::new()),
            star: RefCell::new(Option::None),
            hold: Mutex::new(vec!()),
            seq: IdSeq::new(0),
        }
    }

    pub fn routes(&self) -> usize
    {
        let inner = self.inner.read().unwrap();
        inner.routes.len()
    }

    pub fn set_star_id(&self, node_id: Id)
    {
        self.star.replace(Option::Some(node_id));
    }

    fn star(&self) -> Id
    {
        self.star.borrow().unwrap().clone()
    }

    fn has_route_to_star(&self, star: &Id) -> HasStar {
        let inner = self.inner.read().unwrap();
        inner.has_route_to_star(star)
    }

    pub fn relay_wire(&self, wire: Wire) -> Result<(), Error>
    {
        self.relay_wire_excluding(wire, Option::None)
    }

    pub fn relay_wire_excluding(&self, wire: Wire, exclude: Option<Id>) -> Result<(), Error>
    {
        //println!("routing Wire {}", wire);

        match &wire
        {
            Wire::Search(search) => {
                let routes = {
                    let inner = self.inner.read()?;
                    inner.get_all_routes_excluding(&exclude)
                }?;
                for route in routes
                {
                    route.wire(Wire::Search(search.clone()));
                }
            }
            Wire::Unwind(unwind) => {
                //println!("unwrapped UNWIND.... Forwarding UNWIND {} ", unwind.hops);
                let route = {
                    let inner = self.inner.read()?;
                    match unwind.path.is_empty()
                    {
                        true => {
                            //println!("unwind path is empty");
                            return Err("unwind path is empty".into());
                        }
                        false => {
                            inner.get_route_for_node(unwind.path.last().as_ref().unwrap())
                        }
                    }
                };
                match route
                {
                    Ok(route) => {
                        route.wire(wire)?;
                    }
                    Err(error) => {
                        self.handle_route_error(wire, &error, exclude);
                    }
                }
            }
            Wire::Relay(payload) => {
                let route = {
                    let inner = self.inner.read()?;
                    inner.get_route_for_node(&payload.to)
                };
                match route
                {
                    Ok(route) => {
                        route.wire(wire)?;
                    }
                    Err(error) => {
                        self.handle_route_error(wire, &error, exclude);
                        return Err("could not find node when attempting to relay wire TO".into());
                    }
                }
            },
            Wire::Broadcast(broadcast) => {
                let routes = {
                    let inner = self.inner.read()?;
                    inner.get_all_watching_routes_excluding(&broadcast.watch(), &exclude)?
                };

                for route in routes
                {
                   route.wire(Wire::Broadcast(broadcast.clone()));
                }
            },
            _ => {
                return Err("can only relay Wire::Relay, Wire::Broadcast , Wire::Search & Wire::Unwrap type wires".into());
            }
        }
        Ok(())
    }

    pub fn may_have_route_excluding(&self, star_id: &Id, exclude: Option<Id>) -> bool
    {
        let routes = {
            let inner = self.inner.read().unwrap();
            inner.get_all_routes_excluding(&exclude).unwrap()
        };

        for route in routes
        {
            match route.has_route_to_star(star_id)
            {
                HasStar::Yes(_) => {
                    return true;
                }
                HasStar::No => {
                    // do nothing
                }
                HasStar::Unknown => {
                    return true;
                }
            };
        }
        false
    }


    fn release_hold(&self, star: Id)
    {
        //println!("RELEASING HOLD");
        let releases: Vec<(Wire, Option<Id>)> = {
            let mut hold = self.hold.lock().expect("hold lock should not be poisoned");
         //   println!("holds: {}", hold.len());
            hold.drain(..).collect()
        };
//        println!("releases : {}", releases.len());

        for (wire, exclude) in releases
        {
            match &wire
            {
                Wire::Relay(relay) => {

                    if relay.to == star
                    {
                        self.relay_wire_excluding(wire, exclude);
                    }
                }
                Wire::Unwind(unwind) => {
                    if unwind.path.contains(&star )
                    {
                        self.relay_wire_excluding(wire, exclude);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn notify_search_result(&self, unwind: &Unwind, connection: Arc<Connection>)
    {
        match self.notify_search_result_unwind(unwind, connection.clone())
        {
            None => {}
            Some(unwind) => {



                self.relay_wire_excluding(Wire::Unwind(unwind.clone()), connection.get_remote_node());

                match &unwind.payload{
                    UnwindPayload::SearchFoundResult(result) => {
                        if result.sought.is_some()
                        {
                            self.release_hold(result.sought.unwrap());
                        }
                    }
                    UnwindPayload::SearchNotFoundResult(_) => {}
                }
            }
        }
    }

    fn notify_search_result_unwind(&self, unwind: &Unwind, connection: Arc<Connection>) -> Option<Unwind>
    {
        let  mut inner = self.inner.write().expect("must have access to inner write lock");

        let unwind = unwind.pop();

        match &unwind.payload {
            UnwindPayload::SearchFoundResult(search) => {
                match search.kind
                {
                    SearchKind::StarId(seek) => {
                        if search.transactions.last().is_some()
                        {
                            let transaction = search.transactions.last().unwrap();
                            // since this is a find, we remove the search watch
                            // there may be more finds, but the search watch is only used to
                            // report unfounds once all routes have been searched
                            inner.searches.remove(transaction);
                            connection.add_found_node(search.sought.unwrap().clone(), NodeFind::new(unwind.hops, 0));
                            return Option::Some(unwind);
                        } else {
                            println!("ERROR! Expected transaction at top of stack!");
                        }
                    }
                    _ => {
                        return Option::Some(unwind);
                    }
                }
            }
            UnwindPayload::SearchNotFoundResult(search) => {
                match search.kind
                {
                    SearchKind::StarId(seek) => {
                        connection.add_unfound_node(search.sought.unwrap().clone());
                        if search.transactions.last().is_some()
                        {
                            let transaction = search.transactions.last().unwrap();

                            let mut search_watcher = inner.searches.get_mut(transaction);
                            if search_watcher.is_some()
                            {
                                let mut search_watcher= search_watcher.unwrap();
                                search_watcher.dec();
                                if search_watcher.count == 0
                                {
                                    return Option::Some(unwind);
                                }
                            }
                        } else {
                            println!("ERROR! Expected transaction at top of stack!");
                        }
                    }
                    _ => {
                        return Option::Some(unwind);
                    }
                }
            }
        }
        return Option::None;
    }


    fn handle_route_error(&self, wire: Wire, route_problem: &RouteProblem, exclude: Option<Id>)
    {
        match route_problem {
            RouteProblem::StarUnknown(star) => {
                self.add_to_hold((wire, exclude));
                let transaction = {
                    let inner = self.inner.write().expect("must be able to get inner lock");
                    let transaction = self.seq.next();
                    let search = Search::new(self.star(), SearchKind::StarId(star.clone()), self.seq.next());
                    let count = inner.routes.len().clone();
                    let mut inner = inner;
                    inner.searches.insert(transaction, SearchWatcher{
                        search: search,
                        count: count
                    });
                    transaction
                };
                self.relay_wire(Wire::Search(Search {
                    from: self.star(),
                    kind: SearchKind::StarId(star.clone()),
                    max_hops: 16,
                    transactions: vec![transaction],
                    hops: vec![self.star()],
                }));
            }
            RouteProblem::StarDoesNotExist(star) => {
                panic!("NODE DOES NOT EXIST {:?}", star)
            }
            NucleusNotKnown => {
                panic!("NUCLEUS UNKNOWN")
            }
        }
    }

    fn add_to_hold(&self, wire: (Wire, Option<Id>))
    {
        let mut hold = self.hold.lock().unwrap();
        hold.push(wire);
    }

    pub fn add_route(&self, route: Arc<dyn ExternalRoute>)
    {
        {
            let mut inner = self.inner.write().expect("must get the inner lock");
            inner.add_route(route)
        }
    }




    fn request_node_id_for_nucleus( &self, nucleus_id: Id)
    {

    }

    fn request_route_for_node_id( &self, node_id: Id)
    {

    }

    /*
    fn flush_hold(&self)
    {
        let wires : Vec<(Wire,Option<Id>)> = {
            let mut hold = self.hold.lock().expect("cannot get hold lock");
            hold.drain(..).collect()
        };

println!("FLUSH HOLD {}", wires.len());
        for (wire,exclude) in wires
        {
            self.relay_wire_excluding(wire,exclude);
        }
    }

     */
}

struct SearchWatcher
{
    search: Search,
    count: usize,
}

impl SearchWatcher
{
    fn dec(&mut self)
    {
        self.count = self.count - 1;
    }
}

/*
impl Route for ExternalRouter
{
    fn relay(&self, message_transport: MessageTransport) -> Result<(), Error> {
        let inner = self.inner.read()?;
        match inner.get_route(&message_transport)
        {
            Ok(route) => {
                route.wire(Wire::MessageTransport(message_transport))
            }
            Err(error) => match error {
                NodeNotKnown(node_id) => {
                    self.add_to_hold(message_transport);
                    self.request_route_for_node_id(node_id);
                    Ok(())
                }
                NucleusNotKnown => {
                    let nucleus_id = message_transport.message.to.tron.nucleus.clone();
                    self.add_to_hold(message_transport);
                    self.request_node_id_for_nucleus(nucleus_id);
                    Ok(())
                }
            }
        }
    }
}
 */





pub struct NodeRouter
{
    internal: InternalRouter,
    external: ExternalRouter
}

impl NodeRouter
{
    pub fn external_routes(&self) -> usize
    {
        self.external.routes()
    }

    pub fn relay_wire(&self, wire: Wire) -> Result<(), Error>
    {
        self.external.relay_wire(wire)
    }

    pub fn relay_wire_excluding(&self, wire: Wire, excluding: Option<Id>) -> Result<(), Error>
    {
        self.external.relay_wire_excluding(wire, excluding)
    }

    pub fn may_have_route_excluding(&self, star: &Id, excluding: Option<Id>) -> bool
    {
        self.external.may_have_route_excluding(star, excluding)
    }

    pub fn add_route(&self, route: Arc<dyn ExternalRoute>)
    {
        self.external.add_route(route);
    }

    pub fn set_node_id(&self, node_id: Id)
    {
        self.external.set_star_id(node_id);
    }

    pub fn notify_found(&self, unwind: &Unwind, connection: Arc<Connection>)
    {
        self.external.notify_search_result(unwind, connection);
    }
}


impl Route for NodeRouter
{
    fn relay(&self, message: Arc<Message>) -> Result<(), Error> {
        let nucleus_id = message.to.tron.nucleus.clone();
        if self.internal.has_nucleus(nucleus_id)
        {
unimplemented!()
//            self.internal.relay(message)?;
        }
        else {
unimplemented!()
//            self.external.relay(MessageTransport::new())?;
        }
        Ok(())
    }


}

impl NodeRouter
{
    pub fn new( )->Self
    {
        NodeRouter {
            internal: InternalRouter::new(),
            external: ExternalRouter::new(),
        }
    }

    pub fn add_external_connection(&self, connection: Arc<Connection>)
    {
        self.external.add_route(connection);
    }


    fn has_route_to_star(&self, star: &Id) -> HasStar {
        self.external.has_route_to_star(star)
    }
}


impl ExternalRoute for Connection
{
    fn get_remote_node(&self)->Option<Id>
    {
        self.get_remote_node_id()
    }

    fn wire(&self, wire: Wire) -> Result<(), Error> {
        self.to_remote(wire)
    }

    fn relay(&self, message_transport: MessageTransport) -> Result<(), Error> {
        self.to_remote(Wire::MessageTransport(message_transport));
        Ok(())
    }

    fn has_route_to_star(&self, node_id: &Id) -> HasStar {
        if self.found_nodes.borrow().contains_key(node_id)
        {
            HasStar::Yes(self.found_nodes.borrow().get(node_id).unwrap().hops.clone())
        } else if self.unfound_nodes.borrow().contains(node_id) {
            HasStar::No
        } else {
            HasStar::Unknown
        }
    }

    fn has_watch( &self, watch: &Watch )->bool
    {
        let watches = self.watches.borrow();
        watches.contains(watch)
    }

}





pub enum Wire
{
    ReportVersion(i32),
    ReportNodeId(Id),
    RequestUniqueSeq,
    ReportUniqueSeq(ReportUniqueSeqPayload),
    //   NodeSearch(NodeSearchPayload),
//   NodeFound(NodeSearchPayload),
//   NodeNotFound(NodeSearchPayload),
    MessageTransport(MessageTransport),
    Search(Search),
    Relay(Relay),
    Unwind(Unwind),
    Broadcast(Broadcast),
    Panic(String)
}


impl fmt::Display for Wire {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let r = match self {
            Wire::ReportVersion(_) => { "ReportVersion" }
            Wire::ReportNodeId(_) => { "ReportNodeId" }
            Wire::RequestUniqueSeq => { "RequestUniqueSeq" }
            Wire::ReportUniqueSeq(_) => { "ReportUniqueSeq" }
            Wire::MessageTransport(_) => { "MessageTransport" }
            Wire::Relay(relay) => { "Relay<>"}
            Wire::Search(_) => { "Search<>"}
            Wire::Broadcast(_) => { "Broadcast<>"}
            Wire::Panic(_) => { "Panic" }
            Wire::Unwind(_) => { "Unwind" }
        };
        write!(f, "{}",r)
    }
}

#[derive(Clone)]
pub enum Broadcast
{
    Nucleus(NucleusBomb)
}

impl Broadcast
{
    pub fn watch(&self)->Watch
    {
        match self
        {
            Broadcast::Nucleus(bomb) => {
                Watch::Nucleus(bomb.nucleus.clone())
            }
        }
    }
}

#[derive(Clone)]
pub struct Search
{
    pub from: Id,
    pub kind: SearchKind,
    pub max_hops: i32,
    pub transactions: Vec<Id>,
    pub hops: Vec<Id>,
}

impl Search
{}

#[derive(Clone)]
pub struct SearchResult
{
    pub sought: Option<Id>,
    pub kind: SearchKind,
    pub transactions: Vec<Id>,
}


#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub enum Watch
{
    Nucleus(Id)
}


#[derive(Clone)]
pub struct SearchNotResultFound
{
    pub sought: Id,
    pub hops: i32,
    pub transaction: Vec<Id>,
}

impl Search {
    pub fn new(from: Id, kind: SearchKind, transaction: Id) -> Self
    {
        let hops = vec![from];
        let transactions = vec![transaction];
        Search {
            from: from,
            kind: kind,
            hops: hops,
            max_hops: 16,
            transactions: transactions,
        }
    }

    pub fn push(self, star: Id, transaction: Id) -> Self
    {
        let mut hops = self.hops.clone();
        let mut transactions = self.transactions.clone();
        hops.push(star);
        transactions.push(transaction);
        Search {
            from: self.from,
            kind: self.kind,
            max_hops: self.max_hops,
            hops: hops,
            transactions: transactions,
        }
    }

    pub fn pop(&mut self)
    {
        self.hops.pop();
        self.transactions.pop();
    }
}



#[derive(Clone,Eq,PartialEq)]
pub enum SearchKind
{
    StarKind(StarKind),
    StarId(Id),
}

impl SearchKind
{
    pub fn matches( &self, star: &Star )->bool
    {
        match self
        {
            SearchKind::StarKind(star_kind) => {
                star.core.kind() == *star_kind
            }
            SearchKind::StarId(star_id) => {
                star.id() == star_id.clone()
            }
        }
    }

    pub fn is_multiple_match(&self) -> bool
    {
        match self
        {
            SearchKind::StarKind(_) => true,
            SearchKind::StarId(_) => false
        }
    }
}


#[derive(Clone)]
pub struct Unwind
{
    pub from: Id,
    pub payload: UnwindPayload,
    pub path: Vec<Id>,
    pub hops: i32,
}

impl Unwind {
    pub fn new(from: Id, payload: UnwindPayload, path: Vec<Id>, hops: i32) -> Self
    {
        Unwind {
            from: from,
            payload: payload,
            hops: hops,
            path: path,
        }
    }

    pub fn pop(&self) -> Self
    {
        let mut path = self.path.clone();
        path.pop();
        Unwind {
            from: self.from.clone(),
            payload: self.payload.clone(),
            hops: self.hops.clone(),
            path: path,
        }
    }
}

#[derive(Clone)]
pub struct Relay
{
    pub from: Id,
    pub to: Id,
    pub payload: RelayPayload,
    pub inform: Option<Id>,
    pub transaction: Option<Id>,
    pub hops: i32
}

impl Relay{

    pub fn to_central( from: Id, payload: RelayPayload )->Self
    {
        Relay::new( from, Id::new(0,0), payload )
    }
    pub fn new( from: Id, to: Id, payload: RelayPayload )->Self
    {
        Relay{
            from: from,
            to: to,
            payload: payload,
            transaction: Option::None,
            inform: Option::None,
            hops: 0
        }
    }

    pub fn inc_hops(self)->Self
    {
        Relay {
            from: self.from,
            to: self.to,
            payload: self.payload,
            transaction: self.transaction,
            inform: self.inform,
            hops: self.hops + 1,
        }
    }

    pub fn reply(self, payload: RelayPayload)->Self
    {
        Relay{
            from: self.to,
            to: self.from,
            payload: payload,
            transaction: self.transaction,
            inform: self.inform,
            hops: 0
        }
    }
}

#[derive(Clone)]
pub enum RelayPayload
{
    Ping(Id),
    Pong(Id),
    RequestUniqueSeq,
    ReportUniqueSeq(ReportUniqueSeqPayload),
    NodeFound(NodeSearchPayload),
    NodeNotFound(NodeSearchPayload),
    SearchNotFound(Search),
    ReportSupervisorAvailable,
    PledgeServices,
    RequestCreateSimulation(ReqCreateSim),
    AssignSimulationToSupervisor(ReportAssignSimulation),
    AssignSimulationToServer(ReportAssignSimulationToServer),
    RequestSupervisorForSim(Id),
    ReportSupervisorForSim(ReportSupervisorForSim),
    NotifySimulationReady(Id),
    RequestNucleus(RequestNucleus),
    AssignNucleus(AssignNucleus),
    RequestNucleusNode(Id),
    ReportNucleusNode(ReportNucleusNodePayload),
    ReportNucleusReady(ReportNucleusReady),
    ReportNucleusDetached(Id),
    Watch(Watch),
    UnWatch(Watch),
    WatchResult(WatchResult),
}

impl RelayPayload
{
    pub fn is_final_transaction(&self)->bool
    {
        match self
        {
            RelayPayload::Pong(_) => true,
            RelayPayload::NodeFound(_) => true,
            RelayPayload::ReportSupervisorForSim(_) => true,
            _ => false
        }
    }
}



#[derive(Clone)]
pub struct ReportNucleusReady
{
    pub sim: Id,
    pub nucleus: Id,
    pub star: Id
}

#[derive(Clone)]
pub struct RequestNucleus
{
    pub sim: Id,
    pub config: Artifact,
    pub listeners: Vec<NucleusReadyListener>
}


#[derive(Clone)]
pub struct AssignNucleus
{
    pub sim: Id,
    pub config: Artifact,
    pub listeners: Vec<NucleusReadyListener>
}

#[derive(Clone)]
pub struct NucleusReadyListener
{
    pub star: Id,
    pub transaction: Id
}

#[derive(Clone)]
pub enum UnwindPayload
{
    SearchFoundResult(SearchResult),
    SearchNotFoundResult(SearchResult),
}

#[derive(Clone)]
pub struct WatchResult
{
  pub kind: WatchResultKind
}

#[derive(Clone)]
pub enum WatchResultKind
{
    Nucleus(NucleusBomb)
}

impl fmt::Display for RelayPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let r = match self {
            RelayPayload::Ping(_) => "Ping",
            RelayPayload::Pong(_) => "Pong",
            RelayPayload::RequestUniqueSeq => "RequestUniqueSeq",
            RelayPayload::ReportUniqueSeq(_) => "ReportUniqueSeq",
            RelayPayload::NodeFound(_) => "NodeFound",
            RelayPayload::NodeNotFound(_) => "NodeNotFound",
            RelayPayload::ReportSupervisorAvailable => "ReportSupervisorAvailable",
            RelayPayload::PledgeServices => "PledgeServices",
            RelayPayload::RequestCreateSimulation(_) => "RequestCreateSimulation",
            RelayPayload::AssignSimulationToSupervisor(_) => "AssignSimulationToSupervisor",
            RelayPayload::AssignSimulationToServer(_) => "AssignSimulationToServer",
            RelayPayload::RequestSupervisorForSim(_) => "RequestSupervisorForSim",
            RelayPayload::ReportSupervisorForSim(_) => "ReportSupervisorForSim",

            RelayPayload::NotifySimulationReady(_) => "NotifySimulationReady",
            RelayPayload::RequestNucleus(_) => "RequestNucleus",
            RelayPayload::AssignNucleus(_) => "AssignNucleus",
            RelayPayload::ReportNucleusReady(_) => "ReportNucleusReady",
            RelayPayload::RequestNucleusNode(_) => "RequestNucleusNode",
            RelayPayload::ReportNucleusNode(_) => "ReportNucleusNode",
            RelayPayload::ReportNucleusDetached(_) => "ReportNucleusDetached",
            RelayPayload::SearchNotFound(_) => "SearchNotFound",

            RelayPayload::Watch(_) => "Watch",
            RelayPayload::UnWatch(_) => "UnWatch",
            RelayPayload::WatchResult(_) => "WatchResult"
        };
        write!(f, "{}", r)
    }
}


#[derive(Clone)]
pub struct ReportAssignSimulationToServer
{
    pub simulation_id: Id,
    pub simulation_config: Artifact,
}

#[derive(Clone)]
pub struct ReqCreateSim
{
    pub simulation: Id,
    pub simulation_config: Artifact,
    pub nearby_supervisors: HashSet<Id>,
}

#[derive(Clone)]
pub struct ReportAssignSimulation
{
    pub simulation_id: Id,
    pub simulation_config: Artifact,
}


#[derive(Clone)]
pub struct ReportSupervisorForSim
{
    pub simulation: Id,
    pub star: Id
}

#[derive(Clone)]
pub struct ReportNucleusNodePayload
{
    pub sim: Id,
    pub node: Id
}


#[derive(Clone,Debug)]
pub struct NodeSearchPayload
{
    pub from: Id,
    pub seeking_id: Id,
    pub hops: i32,
    pub timestamp: i32
}

impl NodeSearchPayload
{

}

pub struct RecentNodeSearch
{
    pub id: Id,
    pub timestamp: u64
}

pub struct RecentConnectionTransaction
{
    pub connection: Arc<Connection>,
    pub timestamp: u64
}

#[derive(Clone)]
pub struct ReportUniqueSeqPayload
{
    pub seq: i64
}

impl ReportUniqueSeqPayload
{
    pub fn new( seq: i64 )->Self
    {
        ReportUniqueSeqPayload{
            seq: seq
        }
    }
}


pub enum RouteProblem
{
    StarUnknown(Id),
    StarDoesNotExist(Id),
    NucleusNotKnown,
}


struct ExternalRouterInner
{
    pub nucleus_to_node_table: HashMap<Id, NucleusRoute>,
    pub routes: Vec<Arc<dyn ExternalRoute>>,
    pub searches: HashMap<Id, SearchWatcher>,
}

impl ExternalRouterInner
{
    pub fn new() -> Self
    {
        ExternalRouterInner {
            nucleus_to_node_table: HashMap::new(),
            routes: vec!(),
            searches: HashMap::new(),
        }
    }

    fn has_route_to_star(&self, star: &Id) -> HasStar {
        let mut unknown = false;
        for route in &self.routes
        {
            match route.has_route_to_star(star)
            {
                HasStar::Yes(hops) => {
                    return HasStar::Yes(hops);
                }
                HasStar::No => {}
                HasStar::Unknown => {
                    unknown = true;
                }
            }
        }
        if unknown
        {
            return HasStar::Unknown;
        } else {
            return HasStar::No;
        }
    }


    pub fn add_route(&mut self, route: Arc<dyn ExternalRoute>)
    {
        self.routes.push(route);
    }

    pub fn remove_route(&mut self, route: Arc<ExternalRouterInner>)
    {
        self.remove_route(route);
    }

    pub fn get_all_routes_excluding(&self,  exclude: &Option<Id> ) ->Result<Vec<Arc<dyn ExternalRoute>>,Error>
    {
        Ok(self.routes.iter().map(|route|route.clone()).filter(|route|match route.get_remote_node(){
            None => {
                return false;
            }
            Some(star) => {
                match exclude{
                    None => {
                        return true;
                    }
                    Some(exclude) => {
                        return *exclude != star;
                    }
                }
            }
        } ).collect())
    }

    pub fn get_all_watching_routes_excluding(&self,  watch: &Watch, exclude: &Option<Id> ) ->Result<Vec<Arc<dyn ExternalRoute>>,Error>
    {
        Ok(self.routes.iter().filter(|route| exclude.is_none() || exclude.unwrap() != route.get_remote_node().unwrap() ).filter(|route| route.has_watch(watch) ).map(|route|route.clone()).collect())
    }


    pub fn get_all_posible_routes_for_node(&self, node: &Id ) ->Result<Vec<Arc<dyn ExternalRoute>>,Error>
    {
        Ok(self.routes.iter().map(|route| route.clone()).filter(|route| match route.has_route_to_star(node) {
            HasStar::Yes(_) => { true }
            HasStar::No => { false }
            HasStar::Unknown => { true }
        }).collect())
    }

    pub fn get_route_for_node( &self, node: &Id )->Result<Arc<dyn ExternalRoute>,RouteProblem>
    {
        let mut best_route: Option<Arc<dyn ExternalRoute>> = Option::None;
        let mut unknown = false;
        let mut existing_hops: i32 = 100_000;
        for route in &self.routes
        {
            best_route = match route.has_route_to_star(node)
            {
                HasStar::Yes(hops) => match &best_route
                {
                    Some(existing_route) => {
                        if hops < existing_hops
                        {
                            existing_hops = hops;
                            Option::Some(route.clone())
                        } else {
                            best_route.clone()
                        }
                    },
                    None => {
                        existing_hops = hops;
                        Option::Some(route.clone())
                    }
                }
                HasStar::No => {
                    println!("FOUND HasNode::No");
                    best_route
                },
                HasStar::Unknown => {
                    unknown = true;
                    best_route
                }
            }
        }

        match best_route
        {
            None => {
                match unknown
                {
                    true => Err(RouteProblem::StarUnknown(node.clone())),
                    false => Err(RouteProblem::StarDoesNotExist(node.clone()))
                }
            }
            Some(route) => {
                Ok(route.clone())
            }
        }
    }

    pub fn get_route( &self, transport: &MessageTransport )->Result<Arc<dyn ExternalRoute>,RouteProblem>
    {
        let node = self.nucleus_to_node_table.get(&transport.message.to.tron.nucleus);
        if node.is_none()
        {
            return Err(RouteProblem::NucleusNotKnown);
        }
        let node = node.unwrap().node_id;

        self.get_route_for_node(&node)
    }
}

pub trait Route
{
    fn relay(&self, message: Arc<Message>) ->Result<(),Error>;
}

pub trait InternalRoute: Route
{
    fn has_nucleus(&self, nucleus_id: &Id)->bool;
}

pub enum HasStar
{
    Yes(i32),
    No,
    Unknown,
}

pub trait ExternalRoute
{
    fn wire(&self, wire: Wire ) ->Result<(),Error>;
    fn relay(&self, message_transport: MessageTransport) -> Result<(), Error>;
    fn has_route_to_star(&self, node_id: &Id) -> HasStar;
    fn get_remote_node(&self) -> Option<Id>;
    fn has_watch(&self, watch: &Watch )->bool;
}


pub trait WireListener
{
    fn on_wire(&self, wire: Wire, connection: Arc<Connection> ) ->Result<(),Error>;
    fn describe(&self ) -> String;
}


pub struct NucleusRoute
{
    pub node_id: Id,
    pub last_used: u64
}



#[cfg(test)]
mod test
{
    use std::cell::{Cell, RefCell};
    use std::io;
    use std::io::Write;
    use std::sync::{Arc, RwLock};

    use mechtron_common::artifact::Artifact;
    use mechtron_common::id::Id;

    use crate::cache::default_cache;
    use crate::cluster::Cluster;
    use crate::transport::{connect, Connection, HasStar, NodeSearchPayload, Relay, RelayPayload, ReportUniqueSeqPayload, ReqCreateSim, Search, SearchKind, Wire, ReportSupervisorForSim, ReportNucleusNodePayload};
    use crate::star::{PanicErrorHandler, Server, Star, StarCore, Supervisor, TransactionResult, TransactionWatcher};

    pub struct TestTransactionWatcher
    {
        pub ready: Cell<Id>,
        pub report_supervisor_for_sim: RefCell<Option<ReportSupervisorForSim>>,
        pub nucleus_node: RefCell<Option<ReportNucleusNodePayload>>
    }

    impl TestTransactionWatcher {
        pub fn new() -> Self
        {
            TestTransactionWatcher
            {
                ready: Cell::new(Id::new(0,0)),
                report_supervisor_for_sim: RefCell::new(Option::None),
                nucleus_node: RefCell::new(Option::None)
            }
        }
    }

    impl TransactionWatcher for TestTransactionWatcher
    {
        fn on_transaction(&self, wire: &Wire, star: &Star) -> TransactionResult {
            println!("TEST TRANSACTION WATCHER WIRE {}", wire);

            match wire {
                Wire::Relay(relay) => {
                    println!("TEST TRANSACTION WATCHER WIRE relay.payloaad {}", relay.payload);
                    match &relay.payload {
                        RelayPayload::NotifySimulationReady(sim_id) => {
                            println!("NOTIFY SIMULATION READY !!!!!  ");
                            self.ready.replace(sim_id.clone() );
                        }
                        RelayPayload::ReportSupervisorForSim(report) => {
                            self.report_supervisor_for_sim.replace(Option::Some(report.clone()));
                        }
                        RelayPayload::ReportNucleusNode(report) => {
                            self.nucleus_node.replace(Option::Some(report.clone()));
                        }
                        _ => {}
                    }
                }
                _ => {
                    println!("SOMETHING ELSE")
                }
            }

            TransactionResult::Inconclusive
        }
    }

    #[test]
    pub fn test_connection()
    {
        let cache = Option::Some(default_cache());
        let central = Arc::new(Star::new(StarCore::Central(RwLock::new(Cluster::new())), cache.clone()));
        let server = Arc::new(Star::new(StarCore::Server(RwLock::new(Server::new())), cache));

        let (mut a, mut b) = connect(central.clone(), server.clone());

        a.set_block(false);
        b.set_block(false);

        assert!( central.is_init() );
        assert!( server.is_init() );

        assert_eq!( b.get_remote_node_id(), *central.id.borrow());
        assert_eq!( a.get_remote_node_id(), *server.id.borrow());
    }


    #[test]
    pub fn test_report_report_node_id_twice()
    {
        let cache = Option::Some(default_cache());
        let central = Arc::new(Star::new(StarCore::Central(RwLock::new(Cluster::new())), cache.clone() ));
        let server = Arc::new(Star::new(StarCore::Server(RwLock::new(Server::new())), cache ));

        let (a, b) = connect(central.clone(), server.clone());

        assert!(central.is_init());
        assert!(server.is_init());

        a.to_remote(Wire::ReportNodeId(Id::new(0, 0))).unwrap();

        assert!(b.is_error());
    }

    #[test]
    pub fn test_node_search()
    {
        let cache = Option::Some(default_cache());
        let central = Arc::new(Star::new(StarCore::Central(RwLock::new(Cluster::new())), cache.clone()));
        let mesh = Arc::new(Star::new(StarCore::Mesh, cache.clone()));
        let a = Arc::new(Star::new(StarCore::Mesh, cache.clone()));
        let b = Arc::new(Star::new(StarCore::Mesh, cache.clone()));
        let c = Arc::new(Star::new(StarCore::Mesh, cache.clone()));
        let c2 = Arc::new(Star::new(StarCore::Mesh, cache.clone()));
        let c3 = Arc::new(Star::new(StarCore::Mesh, cache.clone()));

        connect(central.clone(), mesh.clone());
        connect(mesh.clone(), a.clone());
        connect(mesh.clone(), b.clone());
        connect(mesh.clone(), c.clone());
        connect(c.clone(), c2.clone());
        connect(c2.clone(), c3.clone());

        assert!(central.is_init());
        assert!(mesh.is_init());
        assert!(a.is_init());
        assert!(b.is_init());
        assert!(c.is_init());
        assert!(c2.is_init());
        assert!(c3.is_init());

        match c3.router.has_route_to_star(&a.id())
        {
            HasStar::Yes(_) => {
                assert!(false)
            }
            HasStar::No => {
                assert!(false)
            }
            HasStar::Unknown => {
                assert!(true)
            }
        }


        match c2.router.has_route_to_star(&a.id())
        {
            HasStar::Yes(_) => {
                assert!(false)
            }
            HasStar::No => {
                assert!(false)
            }
            HasStar::Unknown => {
                assert!(true)
            }
        }

        match c.router.has_route_to_star(&a.id())
        {
            HasStar::Yes(_) => {
                assert!(false)
            }
            HasStar::No => {
                assert!(false)
            }
            HasStar::Unknown => {
                assert!(true)
            }
        }


        c3.router.relay_wire(Wire::Relay(Relay::new(c3.id(), a.id(), RelayPayload::Ping(a.id()))));


        match c.router.has_route_to_star(&a.id())
        {
            HasStar::Yes(_) => {
                assert!(true)
            }
            HasStar::No => {
                assert!(false)
            }
            HasStar::Unknown => {
                assert!(false)
            }
        }


        match c2.router.has_route_to_star(&a.id())
        {
            HasStar::Yes(_) => {
                assert!(true)
            }
            HasStar::No => {
                assert!(false)
            }
            HasStar::Unknown => {
                assert!(false)
            }
        }

        match c3.router.has_route_to_star(&a.id())
        {
            HasStar::Yes(_) => {
                assert!(true)
            }
            HasStar::No => {
                assert!(false)
            }
            HasStar::Unknown => {
                assert!(false)
            }
        }
    }


    #[test]
    pub fn test_report_request_unique_seq_after_node_set()
    {
        let cache = Option::Some(default_cache());
        let central = Arc::new(Star::new(StarCore::Central(RwLock::new(Cluster::new())), cache.clone()));
        let server = Arc::new(Star::new(StarCore::Server(RwLock::new(Server::new())), cache));

        let (a, b) = connect(central.clone(), server.clone());

        assert!( central.is_init() );
        assert!( server.is_init() );

        a.to_remote(Wire::RequestUniqueSeq ).unwrap();

        assert!( b.is_error() );
    }

    #[test]
    pub fn test_report_request_unique_seq_twice()
    {
        let cache = Option::Some(default_cache());
        let central = Arc::new(Star::new(StarCore::Central(RwLock::new(Cluster::new())), cache.clone() ));
        let server = Arc::new(Star::new(StarCore::Server(RwLock::new(Server::new())), cache ));

        let (a,b) = connect(central.clone(),server.clone() );

        assert!(b.is_ok());
        a.to_remote(Wire::RequestUniqueSeq);
        assert!( b.is_error() );
    }


    #[test]
    pub fn test_report_report_node_id_not_from_seq()
    {
        let cache = Option::Some(default_cache());
        let central = Arc::new(Star::new(StarCore::Central(RwLock::new(Cluster::new())), cache.clone() ));
        let server = Arc::new(Star::new(StarCore::Server(RwLock::new(Server::new())), cache ));

        let (a,b) = connect(central.clone(),server.clone() );

        a.to_remote(Wire::ReportUniqueSeq(ReportUniqueSeqPayload::new(123)));
        assert!(b.is_ok());
        b.to_remote( Wire::ReportNodeId(Id::new(321,0))) ;
        assert!(a.is_error());

    }

    #[test]
    pub fn test_relay_unique_seq()
    {
        let cache = Option::Some(default_cache());
        let central = Arc::new(Star::new(StarCore::Central(RwLock::new(Cluster::new())), cache.clone() ));
        let mesh= Arc::new(Star::new(StarCore::Mesh, cache.clone() ));
        let server = Arc::new(Star::new(StarCore::Server(RwLock::new(Server::new())), cache ));

        let (central_to_mesh,mesh_to_central) = connect(central.clone(),mesh.clone() );

        assert!( central.is_init() );
        assert!( mesh.is_init() );
        assert!( !server.is_init() );

        let (mesh_to_server,server_to_mesh) = connect(mesh.clone(),server.clone() );

        assert!( central.is_init() );
        assert!( mesh.is_init() );
        assert!( server.is_init() );

    }

    #[test]
    pub fn test_long_relay()
    {
        let cache = Option::Some(default_cache());
        let central = Arc::new(Star::new(StarCore::Central(RwLock::new(Cluster::new())), cache.clone() ));
        let mesh= Arc::new(Star::new(StarCore::Mesh, cache.clone() ));
        let server = Arc::new(Star::new(StarCore::Server(RwLock::new(Server::new())), cache.clone() ));
        let gateway = Arc::new(Star::new(StarCore::Gateway, cache.clone() ));
        let client = Arc::new(Star::new(StarCore::Client, cache.clone() ));

        let (central_to_mesh,mesh_to_central) = connect(central.clone(),mesh.clone() );

        assert!( central.is_init() );
        assert!( mesh.is_init() );

        let (mesh_to_server,server_to_mesh) = connect(mesh.clone(),server.clone() );

        assert!( server.is_init() );

        let (gateway_to_mesh,mesh_to_gateway) = connect(gateway.clone(),mesh.clone() );

        assert!( gateway.is_init() );

        println!("attaching CLIENT");
        let (client_to_gateway,gateway_to_client) = connect(client.clone(),gateway.clone() );

        assert!( client.is_init() );

        client.router.relay_wire(Wire::Relay(
           Relay{
               from: client.id(),
               to: Id::new(0,0),
               payload: RelayPayload::Ping(Id::new(1,2)),
               transaction: Option::None,
               inform: Option::None,
               hops: 0
           }
        ));

    }






    #[test]
    pub fn test_circular_graph()
    {
        let cache = Option::Some(default_cache());
        let central = Arc::new(Star::new(StarCore::Central(RwLock::new(Cluster::new())), cache.clone() ));
        let mesh1= Arc::new(Star::new(StarCore::Mesh, cache.clone() ));
        connect(central.clone(),mesh1.clone() );

        assert!( central.is_init() );
        assert!( mesh1.is_init() );

        let mesh2= Arc::new(Star::new(StarCore::Mesh, cache.clone() ));
        connect(central.clone(),mesh2.clone() );
        assert!( mesh2.is_init() );

        let mesh3= Arc::new(Star::new(StarCore::Mesh, cache.clone() ));
        connect(mesh1.clone(),mesh2.clone() );
        let (_,connection)=connect(mesh1.clone(),mesh3.clone() );
        assert!( mesh3.is_init() );

        // issue a NodeFind for a bogus node
        let result = connection.to_remote(Wire::Search(Search {
            from: mesh3.id(),
            kind: SearchKind::StarId(Id::new(335, 552)),
            max_hops: 16,
            transactions: vec![],
            hops: vec!(),
        }));

        assert!(result.is_ok());

        // issue have a GIANT hop size
        let result = connection.to_remote(Wire::Search(Search {
            from: mesh3.id(),
            kind: SearchKind::StarId(Id::new(335, 552)),
            max_hops: 102_324,
            transactions: vec![],
            hops: vec!(),
        }));
    }

    #[test]
    pub fn test_supervisor()
    {

        let cache = Option::Some(default_cache());
        let central = Arc::new(Star::new(StarCore::Central(RwLock::new(Cluster::new())), cache.clone() ));
        let mesh= Arc::new(Star::new(StarCore::Mesh, cache.clone() ));
        let supervisor = Arc::new(Star::new(StarCore::Supervisor(RwLock::new(Supervisor::new())), cache.clone() ));
        let server = Arc::new(Star::new(StarCore::Server(RwLock::new(Server::new())), cache.clone() ));
        let gateway = Arc::new(Star::new(StarCore::Gateway, cache.clone() ));
        let client = Arc::new(Star::new(StarCore::Client, cache.clone() ));

        central.error_handler.replace(Box::new(PanicErrorHandler::new()) );
        mesh.error_handler.replace(Box::new(PanicErrorHandler::new()) );
        supervisor.error_handler.replace(Box::new(PanicErrorHandler::new()) );
        server.error_handler.replace(Box::new(PanicErrorHandler::new()) );
        gateway.error_handler.replace(Box::new(PanicErrorHandler::new()) );
        client.error_handler.replace(Box::new(PanicErrorHandler::new()) );

        let (central_to_mesh,mesh_to_central) = connect(central.clone(),mesh.clone() );
        connect(supervisor.clone(),mesh.clone() );

        assert!( central.is_init() );
        assert!( mesh.is_init() );

        let (mesh_to_server,server_to_mesh) = connect(mesh.clone(),server.clone() );

        assert!( server.is_init() );

        let (gateway_to_mesh,mesh_to_gateway) = connect(gateway.clone(),mesh.clone() );

        assert!( gateway.is_init() );

        println!("attaching CLIENT");
        let (client_to_gateway,gateway_to_client) = connect(client.clone(),gateway.clone() );

        assert!( client.is_init() );


        println!("CENTRAL AVAIL SUPERVISORS...");
        match &central.core
        {
            StarCore::Central(cluster) => {
                let cluster = cluster.read().unwrap();
                assert_eq!(cluster.available_supervisors.len(), 1);
            },
            _ => { assert!(false) }
        }

        println!("SERVER NEAREST SUPERVISOR...");
        assert_eq!(server.nearest_supervisors.lock().unwrap().len(), 1);
        match &server.core
        {
            StarCore::Server(server) => {
                let server = server.read().unwrap();
                assert!(server.supervisor.is_some());
            }
            _ => {}
        }

        println!("SUPERVISORS ...");
        match &supervisor.core
        {
            StarCore::Supervisor(supervisor) => {
                let supervisor = supervisor.read().unwrap();
                assert_eq!(supervisor.servers.len(), 1);
            }
            _ => {}
        }

        println!("CLIENT ID {:?}", client.id());

        let sim_artifact = Artifact::from("mechtron.io:examples:0.0.1:/hello-world/simulation.yaml:sim").unwrap();
        cache.unwrap().configs.cache(&sim_artifact).unwrap();
        let watcher = Arc::new(TestTransactionWatcher::new());
        client.request_create_simulation(sim_artifact, watcher.clone());

        println!("SIMULATILN ID IS: {:?}",watcher.ready.get());
        let sim_id = watcher.ready.get().clone();

        let watcher = Arc::new(TestTransactionWatcher::new());
        client.request_simulation_supervisor(sim_id, watcher.clone());
        assert!( watcher.report_supervisor_for_sim.borrow().is_some());

//        let watcher = Arc::new(TestTransactionWatcher::new());
//        client.request_nucleus_star(sim_id, watcher);

    }
}


