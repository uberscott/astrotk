use std::sync::{Arc, RwLock, Mutex, Weak};

use mechtron_common::message::{Message, MessageTransport};

use crate::node::Node;
use crate::error::Error;
use mechtron_common::id::Id;
use std::collections::{HashMap, HashSet};
use crate::network::RouteProblem::{NodeNotKnown, NucleusNotKnown};
use std::cell::{Cell, RefCell, RefMut};
use crate::router::NetworkRouter;
use std::fmt;

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

    a.init2();
    b.init2();

    (a,b)
}

#[derive(Clone)]
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
    remote_node_id: Option<Id>,
    this: RefCell<Option<Weak<Connection>>>,
}

impl Connection
{
    pub fn init(&self)
    {
        self.relay( Wire::ReportVersion(PROTOCOL_VERSION) );
    }

    pub fn init2(&self)
    {
        self.local.on_wire( Wire::ReportVersion(PROTOCOL_VERSION), self.this() );
    }


    pub fn new( local: Arc<dyn WireListener>)->Self
    {
        Connection{
            status: RefCell::new(ConnectionStatus::WaitVersion),
            local: local,
            remote: RefCell::new(Option::None),
            this: RefCell::new(Option::None),
            remote_node_id: None
        }
    }

    pub fn relay(&self, wire: Wire) ->Result<(),Error>
    {
        let remote = self.remote.borrow();
        remote.as_ref().unwrap().receive( wire);
        Ok(())
    }

    fn error( &self, message: &str )
    {
        println!("{}",message);
        self.status.replace(ConnectionStatus::Error);
    }

    fn this(&self)->Arc<Connection>
    {
        self.this.borrow().as_ref().unwrap().upgrade().unwrap().clone()
    }

    pub fn receive(&self, wire: Wire)
    {
        let status = { (*self.status.borrow()).clone() };
        match status
        {
            ConnectionStatus::WaitVersion => {
                match wire{
                    Wire::ReportVersion(version) => {
                        if version != PROTOCOL_VERSION
                        {
                            self.error("connection ERROR. did not report the expected VERSION")
                        }
                        else {
                            self.status.replace(ConnectionStatus::WaitNodeId );
                        }
                    }
                    _ => {
                        self.error(format!("connection ERROR. expected Version. Got {}",wire).as_str());
                    }
                }
            }
            ConnectionStatus::WaitNodeId => match &wire{
                Wire::ReportNodeId(remote_node_id) => {
                    self.status.replace( ConnectionStatus::Ready );
                    self.local.on_wire( wire, self.this() );
                },
                Wire::RequestUniqueSeq=>
                    {
                        self.local.on_wire( wire, self.this() );
                    }
                Wire::ReportUniqueSeq(seq)=>
                    {
                        self.local.on_wire( wire, self.this() );
                    },

                _ => {
                    self.error(format!("connection ERROR. expected Report NodeId. Got {}",wire).as_str());
                }
            },

            ConnectionStatus::Ready => {
                self.local.on_wire(wire,self.this());

            }

            ConnectionStatus::Error => {
                println!("Connection is Errored, no further processing.");
                return;
            }
        }
//        let connection = self.this.borrow().as_ref().unwrap().upgrade().unwrap().clone();
//        self.local.on_wire(wire,connection);
    }

    pub fn panic( &self, message: Message )
    {

    }

    pub fn close(&self)
    {

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
    hold: Mutex<Vec<MessageTransport>>
}

impl ExternalRouter
{
    pub fn new()->Self
    {
        ExternalRouter{
            inner: RwLock::new(ExternalRouterInner::new()),
        hold: Mutex::new( vec ! () )
        }
    }

    fn add_to_hold(&self, message_transport: MessageTransport )
    {
        let mut hold = self.hold.lock().unwrap();
        hold.push(message_transport);
    }

    fn request_node_id_for_nucleus( &self, nucleus_id: Id)
    {

    }

    fn request_route_for_node_id( &self, node_id: Id)
    {

    }

    fn flush_hold(&self)
    {
        let transports: Vec<MessageTransport> = {
            let mut hold = self.hold.lock().expect("cannot get hold lock");
            hold.drain(..).collect()
        };

        for transport in transports
        {
unimplemented!();
//            self.relay(transport);
        }
    }

    pub fn add_node_route( &self, node_id: Id, route: Arc<dyn ExternalRoute> )->Result<(),Error>
    {
        {
            let mut inner = self.inner.write()?;
            inner.add_route(node_id, route);
        }

        self.flush_hold();

        Ok(())
    }

    pub fn add_nucleus_to_node( &self, nucleus_id: Id, node_id: Id )->Result<(),Error>
    {
        {
            let mut inner = self.inner.write()?;
            inner.add_nucleus_to_node(nucleus_id, node_id);
        }

        self.flush_hold();

        Ok(())
    }


    pub fn remove_route( &mut self, node_id: &Id )
    {
        let mut inner = self.inner.write().unwrap();
        inner.remove_route(node_id);
    }

    pub fn remove_nucleus( &mut self, nucleus_id: &Id )
    {
        let mut inner = self.inner.write().unwrap();
        inner.remove_nucleus(nucleus_id);
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

    pub fn add_external_connection( &self, node_id: Id, connection: Arc<Connection>)
    {
        self.external.add_node_route(node_id, connection );
    }

}


impl ExternalRoute for Connection
{
    fn wire(&self, wire: Wire) -> Result<(), Error> {
        unimplemented!()
    }

    fn relay(&self, message_transport: MessageTransport) -> Result<(), Error> {
        self.relay(Wire::MessageTransport(message_transport));
        Ok(())
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
   from: Id,
   to: Id,
   wire: Box<Wire>,
   transaction: Id
}

#[derive(Clone,Debug)]
pub struct NodeSearchPayload
{
    pub id: Id,
    pub hops: i32,
    pub timestamp: i32
}

pub struct ReportUniqueSeqPayload
{
    pub seq: i64
}



pub enum RouteProblem
{
    NodeNotKnown(Id),
    NucleusNotKnown
}


struct ExternalRouterInner
{
  pub nucleus_to_node_table: HashMap<Id,Id>,
  pub node_to_route_table: HashMap<Id,Arc<dyn ExternalRoute>>
}

impl ExternalRouterInner
{
    pub fn new()->Self
    {
        ExternalRouterInner {
            nucleus_to_node_table: HashMap::new(),
            node_to_route_table: HashMap::new()
        }
    }

    pub fn add_route( &mut self, node_id: Id, route: Arc<ExternalRoute> )
    {
        self.node_to_route_table.insert(node_id, route );
    }

    pub fn remove_route( &mut self, node_id: &Id )
    {
        self.node_to_route_table.remove(node_id);
    }


    pub fn add_nucleus_to_node( &mut self, nucleus_id: Id, node_id: Id,  )
    {
        self.nucleus_to_node_table.insert(node_id, node_id );
    }

    pub fn remove_nucleus( &mut self, nucleus_id: &Id )
    {
        self.nucleus_to_node_table.remove(nucleus_id);
    }



    pub fn get_route( &self, transport: &MessageTransport )->Result<Arc<dyn ExternalRoute>,RouteProblem>
    {
        let node = self.nucleus_to_node_table.get(&transport.message.to.tron.nucleus);
        if node.is_none()
        {
            return Err(RouteProblem::NucleusNotKnown);
        }
        let node = node.unwrap();

        let route = self.node_to_route_table.get( node );

        if route.is_none()
        {
            return Err(RouteProblem::NodeNotKnown(node.clone()));
        }

        let route = route.unwrap();

        Ok(route.clone())
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

pub trait ExternalRoute
{
    fn wire(&self, wire: Wire ) ->Result<(),Error>;
    fn relay(&self, message_transport: MessageTransport ) ->Result<(),Error>;
}


pub trait WireListener
{
    fn on_wire(&self, wire: Wire, connection: Arc<Connection> ) ->Result<(),Error>;
}




#[cfg(test)]
mod test
{
    use crate::node::{Node, NodeKind};
    use crate::cluster::Cluster;
    use crate::network::connect;
    use std::sync::Arc;
    use crate::cache::default_cache;

    #[test]
    pub fn test_connection()
    {
        let cache = Option::Some(default_cache());
        let central = Arc::new(Node::new(NodeKind::Central(Cluster::new()), cache.clone() ));
        let server = Arc::new(Node::new(NodeKind::Server, cache ));

        let (a,b) = connect(central.clone(),server.clone() );

        assert!( central.is_init() );
        assert!( server.is_init() );
    }
}
