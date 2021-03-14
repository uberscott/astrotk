use std::sync::{Arc, RwLock, Mutex, Weak};

use mechtron_common::message::{Message, MessageTransport};

use crate::node::Node;
use crate::error::Error;
use mechtron_common::id::Id;
use std::collections::{HashMap, HashSet};
use crate::network::RouteProblem::{NodeNotKnown, NucleusNotKnown};
use std::cell::Cell;

static PROTOCOL_VERSION: i32 = 1;

pub struct Tunnel
{
    a: Cell<Option<Arc<Connection>>>,
    b: Cell<Option<Arc<Connection>>>
}

impl Tunnel
{
    pub fn new()->Arc<Self>
    {
        let mut tunnel = Arc::new( Tunnel{
             a: Cell::new(Option::None ),
             b: Cell::new(Option::None )
        } );

        let a = Arc::new(Connection::new( tunnel.clone() ));
        let b = Arc::new(Connection::new( tunnel.clone() ));

        a.arc_ref.replace(Option::Some(Arc::downgrade(&a.clone() )));
        b.arc_ref.replace(Option::Some(Arc::downgrade(&b.clone() )));

        tunnel.a.replace(Option::Some(a) );
        tunnel.b.replace(Option::Some(b) );

        tunnel
    }
}

pub struct Connection
{
    init: bool,
    node_id: Option<Id>,
//    local: Cell<Option<Arc<Node>>>,
    remote: Arc<Tunnel>,
    arc_ref: Cell<Option<Weak<Connection>>>
}

impl Route for Connection
{
    fn node_id(&self) -> Id {
        unimplemented!()
    }

    fn forward(&self, message_transport: MessageTransport) -> Result<(), Error> {
        self.relay( Wire::MessageTransport(message_transport) )
    }
}

impl Connection
{
    pub fn new( tunnel: Arc<Tunnel>)->Self
    {
        Connection{
            init: false,
            node_id: Option::None,
            //local: Cell::new(Option::None),
            remote: tunnel,
            arc_ref: Cell::new(Option::None),
        }
    }
    pub fn relay(&self, command: Wire) ->Result<(),Error>
    {
        unimplemented!("i guess we send the message transport to the adjacent node.... WEEE!")
    }

    pub fn receive( & mut self, command: Wire)
    {
        if !self.init
        {
            match command{
                Wire::ProtocolVersion(pv) => {
                    if( pv != PROTOCOL_VERSION)
                    {
                        self.relay(Wire::Panic("Bad Protocol version".to_string()));
                        self.close();
                    }
                    else {
                        self.init = true;
                    }
                }
                _ => {
                    self.relay(Wire::Panic("Bad Protocol version".to_string()));
                    self.close();
                }
            }
            return;
        }


        //self.local.receive(command, self);
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
   ProtocolVersion(i32),
   RequestUniqueSeq,
   RespondUniqueSeq(i64),
   NodeSearch(GraphSearch),
   NodeFound(GraphSearch),
   MessageTransport(MessageTransport),
   Panic(String)
}

#[derive(Clone)]
pub struct GraphSearch
{
    pub id: Id,
    pub hops: i32
}




pub enum RouteProblem
{
    NodeNotKnown,
    NucleusNotKnown
}




pub struct Router
{
    inner: RwLock<RouterInner>
}

impl Router
{
    pub fn new()->Self
    {
        Router{
            inner: RwLock::new(RouterInner::new() )
        }
    }

    pub fn forward( message_transport: MessageTransport )
    {

    }
}


struct RouterInner
{
  pub nucleus_to_node_table: HashMap<Id,Id>,
  pub node_to_route_table: HashMap<Id,Box<dyn Route>>
}

impl RouterInner
{
    pub fn new()->Self
    {
        RouterInner{
            nucleus_to_node_table: HashMap::new(),
            node_to_route_table: HashMap::new()
        }
    }

    pub fn add_route( &mut self, route: Box<dyn Route> )
    {
        self.node_to_route_table.insert(route.node_id(), route );
    }

    pub fn get_route( &self, transport: &MessageTransport )->Result<&Box<dyn Route+'static>,RouteProblem>
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

pub struct ExternalRoute
{
    pub nodes: HashSet<Id>
}





