use std::sync::{Arc, RwLock, Mutex, Weak};

use mechtron_common::message::{Message, MessageTransport};

use crate::node::Node;
use crate::error::Error;
use mechtron_common::id::Id;
use std::collections::{HashMap, HashSet};
use crate::network::RouteProblem::{NodeUnknown, NucleusNotKnown};
use std::cell::{Cell, RefCell, RefMut};
use crate::router::NetworkRouter;
use std::fmt;
use std::hash::Hash;
use std::borrow::Borrow;

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
    block: Cell<bool>
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
            block: Cell::new(true)
        }
    }

    pub fn to_remote(&self, wire: Wire) ->Result<(),Error>
    {
let remote = self.remote.borrow().as_ref().unwrap().local.clone();
println!("to_remote() {} - {} -> {}", self.local.describe(), wire, remote.describe());
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
println!("ADD FOUND {}",find.hops.clone());
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
                        self.error(format!("connection ERROR. expected Report NodeId. Got {} for {}", wire, self.local.describe()).as_str());
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
    hold: Mutex<Vec<Wire>>,
    node_id: RefCell<Option<Id>>
}

impl ExternalRouter
{
    pub fn new()->Self
    {
        ExternalRouter{
            inner: RwLock::new(ExternalRouterInner::new()),
            node_id: RefCell::new(Option::None),
        hold: Mutex::new( vec ! () )
        }
    }
    pub fn set_node_id( &self, node_id: Id)
    {
        self.node_id.replace(Option::Some(node_id));
    }

    fn node_id(&self)->Id
    {
        self.node_id.borrow().unwrap().clone()
    }

    pub fn notify_found( &self, node_id: &Id)
    {
        let releases = {
            let mut hold = self.hold.lock().expect("hold lock should not be poisoned");
            let mut rtn = vec!();
            for wire in hold.drain(..)
            {
                rtn.push(wire)
            }
            rtn
        };

        for wire in releases
        {
            self.relay_wire(wire);
        }
    }

    pub fn relay_wire(&self, wire: Wire )->Result<(),Error>
    {
        match &wire
        {
            Wire::Relay(payload) => {
                let route = {
                    let inner = self.inner.read()?;
                    inner.get_route_for_node(&payload.to)
                };
                match route
                {
                    Ok(route) => {
println!("sending wire to route: {:?}",route.get_remote_node());
                        route.wire(wire)?;
                    }
                    Err(error) => {
                        match error{
                            RouteProblem::NodeUnknown(node) => {
                                self.add_to_hold(wire);
                                self.relay_wire(Wire::NodeSearch(NodeSearchPayload{
                                    to: None,
                                    from: self.node_id(),
                                    seeking_id: node.clone(),
                                    hops: 0,
                                    timestamp: 0
                                }));
                            }
                            RouteProblem::NodeDoesNotExist(node) => {
                                println!("NODE DOES NOT EXIST")
                            }
                            NucleusNotKnown => {
                                panic!("NUCLEUS UNKNOWN")
                            }
                        }
                        return Err("could not find node when attempting to relay wire TO".into());
                    }
                }
            },
            Wire::NodeSearch(payload)=>
            {
                let routes = {
                    let inner = self.inner.read()?;
                    inner.get_all_posible_routes_for_node(&payload.seeking_id )
                };
                match routes
                {
                    Ok(routes)=> {
                        for route in routes
                        {
                            route.wire(Wire::NodeSearch(payload.clone()));
                        }
                    }
                    Err(error)=>{
                        println!("{}",error);
                    }
                }
            }

            _ => {
                return Err("can only relay Wire::Relay type wires".into());
            }
        }
        Ok(())
    }

    fn add_to_hold(&self, wire: Wire)
    {
        let mut hold = self.hold.lock().unwrap();
        hold.push(wire);
    }

    pub fn add_route( &self, route: Arc<dyn ExternalRoute>)
    {
        let mut inner = self.inner.write().expect("must get the inner lock");
        inner.add_route(route)
    }

    fn request_node_id_for_nucleus( &self, nucleus_id: Id)
    {

    }

    fn request_route_for_node_id( &self, node_id: Id)
    {

    }

    fn flush_hold(&self)
    {
        let wires : Vec<Wire> = {
            let mut hold = self.hold.lock().expect("cannot get hold lock");
            hold.drain(..).collect()
        };

        for wire in wires
        {
            self.relay_wire(wire);
        }
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
    pub fn relay_wire(&self, wire: Wire )->Result<(),Error>
    {
        self.external.relay_wire(wire)
    }

    pub fn add_route( &self, route: Arc<dyn ExternalRoute>)
    {
        self.external.add_route(route);
    }

    pub fn set_node_id( &self, node_id: Id)
    {
        self.external.set_node_id(node_id);
    }

    pub fn notify_found( &self, node_id: &Id)
    {
        self.external.notify_found(node_id);
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
        NodeRouter{
            internal: InternalRouter::new(),
            external: ExternalRouter::new(),
        }
    }

    pub fn add_external_connection( &self, connection: Arc<Connection>)
    {
        self.external.add_route(connection );
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

    fn has_node(&self, node_id: &Id) -> HasNode {
        if self.found_nodes.borrow().contains_key(node_id)
        {
            HasNode::Yes(self.found_nodes.borrow().get(node_id).unwrap().hops.clone())
        }
        else if self.unfound_nodes.borrow().contains(node_id){
            HasNode::No
        }
        else
        {
            HasNode::Unknown
        }
    }
}





pub enum Wire
{
   ReportVersion(i32),
   ReportNodeId(Id),
   RequestUniqueSeq,
   ReportUniqueSeq(ReportUniqueSeqPayload),
   NodeSearch(NodeSearchPayload),
   NodeFound(NodeSearchPayload),
   NodeNotFound(NodeSearchPayload),
   MessageTransport(MessageTransport),
   Relay(RelayPayload),
   Panic(String)
}


impl fmt::Display for Wire {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let r = match self {
            Wire::ReportVersion(_) => { "ReportVersion" }
            Wire::ReportNodeId(_) => { "ReportNodeId" }
            Wire::RequestUniqueSeq => { "RequestUniqueSeq" }
            Wire::ReportUniqueSeq(_) => { "ReportUniqueSeq" }
            Wire::NodeSearch(_) => { "NodeSearch" }
            Wire::NodeNotFound(_) => { "NodeNotFound" }
            Wire::NodeFound(_) => { "NodeFound" }
            Wire::MessageTransport(_) => { "MessageTransport" }
            Wire::Relay(relay) => { "Relay<>"}
            Wire::Panic(_) => { "Panic" }
        };
        write!(f, "{}",r)
    }
}

pub struct RelayPayload
{
   pub from: Id,
   pub to: Id,
   pub wire: Box<Wire>,
   pub transaction: Id,
   pub hops: i32
}



#[derive(Clone,Debug)]
pub struct NodeSearchPayload
{
    pub to: Option<Id>,
    pub from: Id,
    pub seeking_id: Id,
    pub hops: i32,
    pub timestamp: i32
}

impl NodeSearchPayload
{
   pub fn reverse(&mut self, from: Id)
   {
       self.to = Option::Some(self.from.clone());
       self.from = from;
   }
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
    NodeUnknown(Id),
    NodeDoesNotExist(Id),
    NucleusNotKnown
}


struct ExternalRouterInner
{
  pub nucleus_to_node_table: HashMap<Id,NucleusRoute>,
  pub routes: Vec<Arc<dyn ExternalRoute>>
}

impl ExternalRouterInner
{
    pub fn new()->Self
    {
        ExternalRouterInner {
            nucleus_to_node_table: HashMap::new(),
            routes: vec!()
        }
    }

    pub fn add_route( &mut self, route: Arc<dyn ExternalRoute> )
    {
        self.routes.push(route );
    }

    pub fn remove_route( &mut self, route: Arc<ExternalRouterInner> )
    {
        self.remove_route(route);
    }

    pub fn get_all_posible_routes_for_node(&self, node: &Id ) ->Result<Vec<Arc<dyn ExternalRoute>>,Error>
    {
        Ok(self.routes.iter().map(|route|route.clone()).filter(|route|match route.has_node(node){
            HasNode::Yes(_) => {true}
            HasNode::No => {false}
            HasNode::Unknown => {true}
        } ).collect())
    }

    pub fn get_route_for_node( &self, node: &Id )->Result<Arc<dyn ExternalRoute>,RouteProblem>
    {
        let mut best_route: Option<Arc<dyn ExternalRoute>> = Option::None;
        let mut unknown = false;
        let mut existing_hops: i32 = 100_000;
        for route in &self.routes
        {
            best_route = match route.has_node( node )
            {
                HasNode::Yes(hops) => match &best_route
                {
                    Some(existing_route)=>{
                        if hops < existing_hops
                        {
                            existing_hops = hops;
                            Option::Some(route.clone())
                        }
                        else {
                            best_route.clone()
                        }
                    },
                    None=>{
                        existing_hops = hops;
                        Option::Some(route.clone())
                    }
                }
                HasNode::No => best_route,
                HasNode::Unknown => {
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
                    true => Err(RouteProblem::NodeUnknown(node.clone())),
                    false => Err(RouteProblem::NodeDoesNotExist(node.clone()))
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

pub enum HasNode
{
    Yes(i32),
    No,
    Unknown
}

pub trait ExternalRoute
{
    fn wire(&self, wire: Wire ) ->Result<(),Error>;
    fn relay(&self, message_transport: MessageTransport ) ->Result<(),Error>;
    fn has_node(&self, node_id:&Id)->HasNode;
    fn get_remote_node(&self)->Option<Id>;
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
    use crate::node::{Node, NodeKind};
    use crate::cluster::Cluster;
    use crate::network::{connect, Wire, ReportUniqueSeqPayload};
    use std::sync::Arc;
    use crate::cache::default_cache;
    use mechtron_common::id::Id;

    #[test]
    pub fn test_connection()
    {
        let cache = Option::Some(default_cache());
        let central = Arc::new(Node::new(NodeKind::Central(Cluster::new()), cache.clone() ));
        let server = Arc::new(Node::new(NodeKind::Server, cache ));

        let (mut a,mut b) = connect(central.clone(),server.clone() );

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
        let central = Arc::new(Node::new(NodeKind::Central(Cluster::new()), cache.clone() ));
        let server = Arc::new(Node::new(NodeKind::Server, cache ));

        let (a,b) = connect(central.clone(),server.clone() );

        assert!( central.is_init() );
        assert!( server.is_init() );

        a.to_remote(Wire::ReportNodeId(Id::new(0, 0)) ).unwrap();

        assert!( b.is_error() );
    }

    #[test]
    pub fn test_report_request_unique_seq_after_node_set()
    {
        let cache = Option::Some(default_cache());
        let central = Arc::new(Node::new(NodeKind::Central(Cluster::new()), cache.clone() ));
        let server = Arc::new(Node::new(NodeKind::Server, cache ));

        let (a,b) = connect(central.clone(),server.clone() );

        assert!( central.is_init() );
        assert!( server.is_init() );

        a.to_remote(Wire::RequestUniqueSeq ).unwrap();

        assert!( b.is_error() );
    }

    #[test]
    pub fn test_report_request_unique_seq_twice()
    {
        let cache = Option::Some(default_cache());
        let central = Arc::new(Node::new(NodeKind::Central(Cluster::new()), cache.clone() ));
        let server = Arc::new(Node::new(NodeKind::Server, cache ));

        let (a,b) = connect(central.clone(),server.clone() );

        assert!(b.is_ok());
        a.to_remote(Wire::RequestUniqueSeq);
        assert!( b.is_error() );
    }


    #[test]
    pub fn test_report_report_node_id_not_from_seq()
    {
        let cache = Option::Some(default_cache());
        let central = Arc::new(Node::new(NodeKind::Central(Cluster::new()), cache.clone() ));
        let server = Arc::new(Node::new(NodeKind::Server, cache ));

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
        let central = Arc::new(Node::new(NodeKind::Central(Cluster::new()), cache.clone() ));
        let mesh= Arc::new(Node::new(NodeKind::Mesh , cache.clone() ));
        let server = Arc::new(Node::new(NodeKind::Server, cache ));

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
        let central = Arc::new(Node::new(NodeKind::Central(Cluster::new()), cache.clone() ));
        let mesh= Arc::new(Node::new(NodeKind::Mesh , cache.clone() ));
        let server = Arc::new(Node::new(NodeKind::Server, cache.clone() ));
        let gateway = Arc::new(Node::new(NodeKind::Gateway, cache.clone() ));
        let client = Arc::new(Node::new(NodeKind::Client, cache.clone() ));

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

    }
}


