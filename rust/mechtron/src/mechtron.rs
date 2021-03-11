use mechtron_common::state::{State, ReadOnlyState};
use mechtron_common::message::{Message, MessageBuilder};
use mechtron_common::error::Error;
use std::sync::MutexGuard;
use mechtron_common::id::{Id, MechtronKey};

#[derive(Clone)]
pub enum Response
{
    None,
    Messages(Vec<MessageBuilder>)
}

#[derive(Clone)]
pub struct Context
{
    pub key: MechtronKey,
    pub phase: String,
    pub cycle: i64
}

pub enum MessageHandler
{
    None,
    Handler( fn( context: &Context, state: &mut MutexGuard<State>, message: Vec<Message>)->Result<Response,Error> )
}

pub enum ExtraCyclicMessageHandler
{
    None,
    Handler( fn( context: &Context, state: &ReadOnlyState, message: Vec<Message>)->Result<Response,Error> )
}

pub trait Mechtron
{
    fn on_create( &self, state: &mut MutexGuard<State>, create_message: &Message ) -> Result<Response, Error>;

    fn on_update( &self, state: &mut MutexGuard<State> ) -> Result<Response,Error>;

    fn on_messages( &self, port: &str ) -> Result<MessageHandler,Error>;

    fn on_extras(&self, port: &str) -> Result<ExtraCyclicMessageHandler, Error>;
}

pub struct BlankMechtron
{
    context: Context
}

impl BlankMechtron{
    pub fn new(context:Context)->Self
    {
        BlankMechtron{
            context:context
        }
    }
}

impl Mechtron for BlankMechtron
{
    fn on_create(&self, state: &mut MutexGuard<State>, create_message: &Message) -> Result<Response, Error> {
        Ok(Response::None)
    }

    fn on_update(&self, state: &mut MutexGuard<State>) -> Result<Response, Error> {
        Ok(Response::None)
    }

    fn on_messages(&self, port: &str) -> Result<MessageHandler, Error> {
        Ok(MessageHandler::None)
    }

    fn on_extras(&self,  port: &str) -> Result<ExtraCyclicMessageHandler, Error> {
        Ok(ExtraCyclicMessageHandler::None)
    }
}

