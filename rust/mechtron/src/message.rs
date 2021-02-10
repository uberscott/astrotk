use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Instant;

use mechtron_common::message::Cycle;
use mechtron_common::message::Message;

use mechtron_common::revision::Revision;
use std::sync::{RwLock, Arc, Mutex};
use crate::content::TronKey;
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
    futures: HashMap<TronKey,RwLock<MessageFutures<'a>>>
}

impl <'a> MessageStore<'a>
{
    pub fn new()->Self
    {
        MessageStore{
            futures: HashMap::new()
        }
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

