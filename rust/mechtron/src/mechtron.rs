use mechtron_common::state::{State, ReadOnlyState};
use mechtron_common::message::{Message, MessageBuilder};
use mechtron_common::error::Error;
use std::sync::MutexGuard;
use mechtron_common::id::{Id, MechtronKey};
use mechtron_common::mechtron::Context;
use mechtron_common::message::DeliveryMoment::ExtraCyclic;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone)]
pub enum Response
{
    None,
    Messages(Vec<MessageBuilder>)
}

pub enum MessageHandler
{
    None,
    Handler( fn( context: &Context, state: &mut State, message: Message)->Result<Response,Error> )
}

pub trait Mechtron
{
    fn create( &mut self, create_message: &Message ) -> Result<Response, Error>;

    fn update( &mut self ) -> Result<Response,Error>;

    fn message( &mut self, port: &str ) -> Result<MessageHandler,Error>;

    fn extra(&self, port: &str) -> Result<MessageHandler, Error>;
}



pub struct BlankMechtron
{
    context: Context,
    state: Rc<RefCell<Option<Box<State>>>>
}

impl BlankMechtron{
    pub fn new(context:Context, state:Rc<RefCell<Option<Box<State>>>>)->Self
    {
        BlankMechtron{
            context:context,
            state : state
        }
    }
}

impl Mechtron for BlankMechtron
{
    fn create(&mut self, create_message: &Message) -> Result<Response, Error> {
        Ok(Response::None)
    }

    fn update(&mut self) -> Result<Response, Error> {
        Ok(Response::None)
    }

    fn message(&mut self, port: &str) -> Result<MessageHandler, Error> {
        Ok(MessageHandler::None)
    }

    fn extra(&self, port: &str) -> Result<MessageHandler, Error> {
        Ok(MessageHandler::None)
    }
}

