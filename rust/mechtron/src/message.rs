use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Instant;

use mechtron_common::message::Cycle;
use mechtron_common::message::Message;

use mechtron_common::revision::Revision;
use std::sync::{RwLock, Arc, Mutex};
use crate::content::TronKey;
use std::error::Error;
use mechtron_common::id::{ContentKey, TronKey, Revision, DeliveryMomentKey};

struct MessageChamber {
    key: TronKey,
    messages: HashMap<DeliveryMomentKey,RwLock<Vec<MessageDelivery>>>
}

struct MessageDelivery
{
    received: Instant,
    message: Message
}

pub struct MessagingStructure
{
    chambers: HashMap<TronKey,RwLock<MessageChamber>>,
    pipeline: Arc<MessagePipeline>
}

impl MessagingStructure
{
    pub fn new()->Self
    {
        MessagingStructure {
            chambers: HashMap::new(),
            pipeline: Arc::new(MessagePipeline::new() )
        }
    }

    pub fn create( &mut self, tron_id: TronKey )->Result<(),Box<dyn Error>>
    {
        if self.chambers.contains_key(&tron_id )
        {
            return Err(format!("MessageStore already contains tron_id {:?} ",tron_id).into());
        }

        self.chambers.insert(tron_id, RwLock::new(MessageChamber::new()));

        return Ok(());
    }

    pub fn cyclic_intake(&self ) ->Arc<dyn MessageIntake>
    {
        return self.pipeline.clone();
    }

    pub fn flood(&mut self)
    {
        let messages = self.pipeline.flood();
    }
}


pub struct MessagePipeline
{
    pipeline: Mutex<Vec<MessageDelivery>>
}

impl MessageIntake for MessagePipeline{

    fn intake(&mut self, message: Message) -> Result<(), Box<dyn Error>> {
        let mut pipeline = self.pipeline.lock()?;
        let delivery = MessageDelivery{
            received: Instant::now(),
            message: message
        };
        pipeline.push(delivery );
        Ok(())
    }
}

impl MessagePipeline{

    pub fn new()->Self{
       MessagePipeline{
           pipeline: Mutex::new(vec!() )
       }
    }

    pub fn flood(&mut self)->Vec<MessageDelivery>
    {
        let mut pipeline = self.pipeline.lock()?;
        let mut rtn = vec!();
        while let Some(delivery)= pipeline.pop()
        {
            rtn.push(delivery );
        }
        return rtn;
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

