use std::sync::{Arc, RwLock, Mutex, Weak};

use mechtron_common::message::{Message, MessageTransport};

use crate::node::Node;
use crate::error::Error;
use mechtron_common::id::Id;
use std::collections::{HashMap, HashSet};
use crate::network::RouteProblem::{NodeNotKnown, NucleusNotKnown};
use std::cell::{Cell, RefCell};
use crate::router::NetworkRouter;

static PROTOCOL_VERSION: i32 = 100_001;


pub fn connect( a: Arc<dyn WireListener>, b: Arc<dyn WireListener> )->(Arc<Connection>,Arc<Connection>)
{
    let mut a = Arc::new(Connection::new(a));
    let mut b= Arc::new(Connection::new(b));
    a.remote.replace(Option::Some( b.clone() ));
    b.remote.replace(Option::Some( a.clone() ));

    a.init();
    b.init();

    (a,b)
}

pub enum ConnectionStatus{
    WaitVersion,
    WaitNodeId,
    Ready,
    Error
}

pub struct Connection
{
    status: ConnectionStatus,
    local: Arc<dyn WireListener>,
    remote: RefCell<Option<Arc<Connection>>>,
    remote_node_id: Option<Id>
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
            if route.has_nucleus(nucleus_id)
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

    pub fn add_to_hold(&self, message_transport: MessageTransport )
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

}

impl Route for ExternalRouter
{
    fn forward(&self, message_transport: MessageTransport) -> Result<(), Error> {
        let inner = self.inner.read()?;
        match inner.get_route(&message_transport)
        {
            Ok(route) => {
                route.relay(Wire::MessageTransport(message_transport))
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





pub struct NodeRouter
{
    internal: InternalRouter,
//    outer: Router<NetworkRoute>
}


impl Route for NodeRouter
{
    fn forward(&self, message_transport: MessageTransport) -> Result<(), Error> {
        let nucleus_id = message_transport.message.to.tron.nucleus.clone();
        if self.internal.has_nucleus(nucleus_id)
        {
            Ok(self.forward(message_transport))
        }
        else {
            unimplemented!()
        }
    }
}


impl ExternalRoute for NodeRouter
{
    fn relay(&self, wire: Wire) -> Result<(), Error> {
        match wire{
            Wire::MessageTransport(transport) =>
                self.forward(transport),
            _ => {
                unimplemented!();
            }
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
        }
    }
    pub fn forward( &self, message_transport: MessageTransport )
    {
        // test if inner contains nucleus and if so forward to inner

        // otherwise forward to outer
    }
}


impl Route for Connection
{

    fn forward(&self, message_transport: MessageTransport) -> Result<(), Error> {
        self.relay(Wire::MessageTransport(message_transport));
        Ok(())
    }
}

impl Connection
{
    pub fn init(&self)
    {
        self.relay( Wire::ReportVersion(PROTOCOL_VERSION) );
    }

    pub fn new( local: Arc<dyn WireListener>)->Self
    {
        Connection{
            status: ConnectionStatus::WaitVersion,
            local: local,
            remote: RefCell::new(Option::None),
            remote_node_id: None
        }
    }
    pub fn relay(&self, wire: Wire) ->Result<(),Error>
    {
        unimplemented!();
//        self.remote.borrow().as_ref().unwrap().receive( wire);
        Ok(())
    }

    pub fn receive( &mut self, command: Wire)
    {
        match self.status
        {
            ConnectionStatus::WaitVersion => {
                match command{
                    Wire::ReportVersion(v) => {
                        if v == PROTOCOL_VERSION{
                            self.status=ConnectionStatus::WaitNodeId
                        }
                        else {
                            self.status=ConnectionStatus::Error;
                        }
                    }
                    _ => {
                        self.status=ConnectionStatus::Error;
                    }
                }
            }
            ConnectionStatus::WaitNodeId => {
                match command{
                    Wire::ReportNodeId(id) => {
                        self.remote_node_id = Option::Some(id);
                        self.status = ConnectionStatus::Ready
                    }
                    Wire::RequestUniqueSeq => {
                        self.relay(command);
                    }
                    _  => {
                        self.status = ConnectionStatus::Error;
                    }
                }
            }
            ConnectionStatus::Ready => {
                match command{
                    Wire::ReportVersion(_) => {
                        self.status = ConnectionStatus::Error;
                    }
                    Wire::ReportNodeId(_) => {
                        self.status = ConnectionStatus::Error;
                    }
                   Wire::Panic(_) => {
                       self.status = ConnectionStatus::Error;
                   }
                    _=>{
                        self.relay(command);
                    }
                }
            }
            _ => {
                println!("not sure what to do with this STATUS!!!")
            }
        };
    }

    pub fn panic( &self, message: Message )
    {

    }

    pub fn close(&self)
    {

    }
}



pub enum Wire
{
   ReportVersion(i32),
   ReportNodeId(Id),
   RequestUniqueSeq,
   RespondUniqueSeq(i64,Option<Id>),
   NodeSearch(WireGraphSearch),
   NodeFound(WireGraphSearch),
   MessageTransport(MessageTransport),
   Panic(String)
}

#[derive(Clone)]
pub struct WireGraphSearch
{
    pub id: Id,
    pub hops: i32,
    pub timestamp: i32
}

#[derive(Clone)]
pub struct WireUniqueSeqRequest
{

}




pub enum RouteProblem
{
    NodeNotKnown(Id),
    NucleusNotKnown
}


struct ExternalRouterInner
{
  pub nucleus_to_node_table: HashMap<Id,Id>,
  pub node_to_route_table: HashMap<Id,Box<dyn ExternalRoute>>
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

    pub fn add_route( &mut self, route: Box<ExternalRoute> )
    {
        unimplemented!()
     //   self.node_to_route_table.insert(route.node_id(), route );
    }

    pub fn get_route( &self, transport: &MessageTransport )->Result<&Box<dyn ExternalRoute>,RouteProblem>
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

        Ok(&route)
    }
}

pub trait Route
{
    fn forward( &self, message_transport: MessageTransport )->Result<(),Error>;
}

pub trait InternalRoute: Route
{
    fn has_nucleus(&self, nucleus_id: Id)->bool;
}

pub trait ExternalRoute: Route
{
    fn relay( &self, wire: Wire )->Result<(),Error>;
}


pub trait WireListener
{
    fn on_wire(&self, wire: Wire, connection: Arc<Connection> ) ->Result<(),Error>;
}





