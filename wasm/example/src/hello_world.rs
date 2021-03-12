use core::option::Option;
use core::option::Option::{None, Some};
use core::result::Result;
use core::result::Result::{Err, Ok};
use mechtron::CONFIGS;
use mechtron_common::api::CreateApiCallCreateNucleus;
use mechtron_common::artifact::Artifact;
use mechtron_common::error::Error;
use mechtron_common::message::{Cycle, MechtronLayer, Message, MessageBuilder, MessageKind};
use mechtron_common::state::{ReadOnlyState, State};
use mechtron::mechtron::{Mechtron, Response,  MessageHandler};
use std::sync::MutexGuard;
use mechtron_common::mechtron::Context;
use std::rc::Rc;
use std::cell::{Cell, RefCell};
use mechtron_common::logger::log;

pub struct HelloWorldMechtron{
    context: Context,
    state: Rc<RefCell<Option<Box<State>>>>
}

impl HelloWorldMechtron{

    pub fn new(context:Context, state: Rc<RefCell<Option<Box<State>>>>)->Self
    {
        HelloWorldMechtron{
            context: context,
            state: state
        }
    }
}

impl Mechtron for HelloWorldMechtron{

    fn create(&mut self, create_message: &Message) -> Result<Response, Error>
    {
        log( "Hello World! from the HelloWorldMechtron.");
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
