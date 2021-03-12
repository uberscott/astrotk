use mechtron_common::state::{State, ReadOnlyState};
use mechtron_common::message::{Message, MessageBuilder};
use mechtron_common::error::Error;
use std::sync::MutexGuard;
use mechtron_common::id::{Id, MechtronKey};
use mechtron_common::mechtron::Context;
use mechtron_common::message::DeliveryMoment::ExtraCyclic;
use std::cell::{Cell, RefCell};
use crate::membrane::StateLocker;

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
    fn create( &self, create_message: &Message ) -> Result<Response, Error>;

    fn update( &self ) -> Result<Response,Error>;

    fn message( &self, port: &str ) -> Result<MessageHandler,Error>;

    fn extra(&self, port: &str) -> Result<MessageHandler, Error>;

    fn state(&self) ->&StateLocker;
}



pub struct BlankMechtron
{
    context: Context,
    state_locker: StateLocker
}

impl BlankMechtron{
    pub fn new(context:Context, state_locker:StateLocker)->Self
    {
        BlankMechtron{
            context:context,
            state_locker: state_locker
        }
    }
}

impl Mechtron for BlankMechtron
{
    fn create(&self, create_message: &Message) -> Result<Response, Error> {
        Ok(Response::None)
    }

    fn update(&self) -> Result<Response, Error> {
        Ok(Response::None)
    }

    fn message(&self, port: &str) -> Result<MessageHandler, Error> {
        Ok(MessageHandler::None)
    }

    fn extra(&self, port: &str) -> Result<MessageHandler, Error> {
        Ok(MessageHandler::None)
    }

    fn state(&self) -> &StateLocker {
        &self.state_locker
    }
}

