use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Instant;

use mechtron_common::message::Cycle;
use mechtron_common::message::Message;

use mechtron_common::revision::Revision;
use std::sync::{RwLock, Arc, Mutex};
use crate::content::TronKey;
use std::error::Error;

struct MessageFutures{
    key: TronKey,
    messages: HashMap<i64,RwLock<Vec<MessageDelivery>>>
}

struct MessageDelivery
{
    received: i64,
    message: Message
}

pub struct MessageStore
{
    futures: HashMap<i64,RwLock<MessageFutures>>
}

impl MessageStore
{
    pub fn new()->Self
    {
        MessageStore{
            futures: HashMap::new()
        }
    }

    pub fn create( &mut self, tron_id: i64 )->Result<(),Box<dyn Error>>
    {
        if self.futures.contains_key(&tron_id )
        {
            return Err(format!("MessageStore already contains tron_id {} ",tron_id).into());
        }

        self.futures.insert(tron_id,RwLock::new(MessageFutures::new()));

        return Ok(());
    }

}



pub trait MessageIntake
{
    fn intake(&mut self, message: Message) -> Result<(),Box<dyn Error>>;
}


pub trait MessageRouter
{
    fn send( &self, messages: Vec<Message> );
}

