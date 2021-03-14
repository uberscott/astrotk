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

pub struct NodeRouter
{
    inner: Router<Route>,
    outer: Router<NetworkRoute>
}

impl NodeRouter
{
    pub fn new( )->Self
    {
        NodeRouter{
            inner: Router::new(),
            outer: Router::new()
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
    fn node_id(&self) -> Id {
        unimplemented!()
    }

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
    NodeNotKnown,
    NucleusNotKnown
}




pub struct Router<R: ?Sized + Route>
{
    inner: RwLock<RouterInner<R>>
}

impl <R: ?Sized+Route> Router<R>
{
    pub fn new()->Self
    {
        Router{
            inner: RwLock::new(RouterInner::new() )
        }
    }

    pub fn forward( &self, message_transport: MessageTransport )
    {

    }

    pub fn add_route(&self, search: WireGraphSearch, route: Arc<dyn Route> )
    {

    }
}

struct RouterInner<R: ?Sized+Route>
{
  pub nucleus_to_node_table: HashMap<Id,Id>,
  pub node_to_route_table: HashMap<Id,Box<R>>
}

impl <R> RouterInner<R> where R: ?Sized+Route
{
    pub fn new()->Self
    {
        RouterInner{
            nucleus_to_node_table: HashMap::new(),
            node_to_route_table: HashMap::new()
        }
    }

    pub fn add_route( &mut self, route: Box<R> )
    {
        self.node_to_route_table.insert(route.node_id(), route );
    }

    pub fn get_route( &self, transport: &MessageTransport )->Result<&Box<R>,RouteProblem>
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
            return Err(RouteProblem::NodeNotKnown);
        }

        let route = route.unwrap();

        Ok(&route)
    }
}

pub trait Route
{
    fn node_id(&self)->Id;
    fn forward( &self, message_transport: MessageTransport )->Result<(),Error>;
}

pub trait NetworkRoute: Route
{
    fn relay( &self, wire: Wire )->Result<(),Error>;
}

pub struct ExternalRoute
{
    pub nodes: HashSet<Id>
}

pub trait WireListener
{
    fn wire( &self, wire: Wire, connection: Arc<Connection> )->Result<(),Error>;
}





