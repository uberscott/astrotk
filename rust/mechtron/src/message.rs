use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Instant;

use mechtron_common::message::Cycle;
use mechtron_common::message::Message;

use mechtron_common::revision::Revision;
use std::sync::{RwLock, Arc};
use crate::content::TronKey;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use std::error::Error;

struct MessageFutures<'a>{
    key: TronKey,
    messages: HashMap<i64,RwLock<Vec<MessageDelivery<'a>>>>
}

struct MessageDelivery<'a>
{
    received: i64,
    message: Message<'a>
}

pub struct MessageStore<'a>
{
    sender: Arc<Sender<Message<'a>>>,
    receiver: Arc<Receiver<Message<'a>>>,
    futures: HashMap<TronKey,RwLock<MessageFutures<'a>>>
}

impl <'a> MessageStore<'a>
{
    pub fn new()->Self
    {
        let (sender, receiver) = mpsc::channel::<Message<'a>>(128);
        MessageStore{
            sender: Arc::new(sender ),
            receiver: Arc::new( receiver ),
            futures: HashMap::new()
        }
    }

    pub fn intake( &self )->Box<dyn MessageIntake>
    {
        return Box::new(MessageChamber { sender: self.sender.clone() });
    }

    pub fn flood( &mut self ) -> Result<(),Box<dyn Error>>
    {
        // somehow open the receiver up until everything is drained
        Ok(())
    }

}

struct MessageChamber<'a>
{
    sender: Arc<Sender<Message<'a>>>
}


impl <'a> MessageIntake<'a> for MessageChamber<'a>
{
    fn intake(&mut self, message: Message) -> Result<(),Box<dyn Error>>
    {
        self.sender.send(message)?;
    }
}


pub trait MessageIntake<'a>
{
    fn intake(&mut self, message: Message<'a>) -> Result<(),Box<dyn Error>>;
}


pub trait MessageRouter<'a>
{
    fn send( &self, messages: Vec<Message<'a>> );
}

