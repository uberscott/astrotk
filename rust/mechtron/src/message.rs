use std::collections::HashMap;
use std::sync::{RwLock, Arc};
use mechtron_common::id::{Id, Revision, TronKey};
use mechtron_common::message::{Message, Cycle};
use std::error::Error;
use std::time::Instant;
use std::collections::hash_map::RandomState;

use crate::app::SYS;
use crate::nucleus::Nucleus;


pub trait MessageRouter
{
    fn send( &mut self, message: Message );
}

pub struct GlobalMessageRouter
{
    local: LocalMessageRouter
}

impl GlobalMessageRouter
{
    pub fn new()->Self
    {
        GlobalMessageRouter{
            local: LocalMessageRouter{}
        }
    }
}

impl MessageRouter for GlobalMessageRouter
{
    fn send(&mut self, message: Message)
    {
        self.local.send(message);
    }
}

struct LocalMessageRouter
{
}

impl MessageRouter for LocalMessageRouter
{
    fn send(&mut self, message: Message)
    {
        match SYS.local.nuclei.get(&message.to.tron.nucleus_id)
        {
            Ok(mut nucleus) => {
                nucleus.intake( message )?;
            }
            Err(e) => {print!("message failed to be sent: {:?}",e)}
        }
    }
}




struct MessageDelivery
{
    received: Instant,
    cycle: i64,
    message: Arc<Message>
}

impl MessageDelivery {
    fn new( message: Message, context: &dyn IntakeContext )->Self
    {
        MessageDelivery{
            received: Instant::now(),
            cycle: context.head(),
            message: Arc::new(message)
        }
    }
}

pub trait IntakeContext
{
    fn head(&self)->i64;
    fn phase(&self)->u8;
}

pub struct NucleusMessagingStructure
{
    store: RwLock<HashMap<i64,HashMap<TronKey, TronMessagingChamber>>>
}

impl NucleusMessagingStructure
{
   pub fn new()->Self
   {
       NucleusMessagingStructure {
           store: RwLock::new(HashMap::new())
       }
   }

   pub fn intake( &mut self, message: Message, context: &dyn IntakeContext )->Result<(),Box<dyn Error>>
   {
       let delivery = MessageDelivery::new(message,context);

       let mut store = self.store.write()?;

       let desired_cycle = match delivery.message.to.cycle{
          Cycle::Exact(cycle) => {
              cycle
          }
          Cycle::Present => {
              // Nucleus intake is InterCyclic therefore cannot accept present cycles
              contex.head()+1
          }
          Cycle::Next => {
              contex.head()+1
          }
      };

      // at some point we must determine if the nucleus policy allows for message deliveries to this
      // nucleus after x number of cycles and then send a rejection message if needed

       if !store.contains_key(&desired_cycle){
           store.insert(desired_cycle.clone(), HashMap::new() );
       }

       let store = self.store.get(&desired_cycle ).unwrap();

       if !store.contains_key(&delivery.message.to.tron )
       {
           store.inserts(delivery.message.to.tron.clone(), TronMessagingChamber::new() )
       }

       let mut chamber = store.get(&delivery.message.to.tron).unwrap();
       chamber.intake(delivery)?;

       Ok(())
   }

    pub fn query( &self, cycle: &i64 )->Result<Vec<Arc<Message>>,Box<dyn Error>>
    {
        match self.store.get(&cycle )
        {
            None => Ok(vec!()),
            Some(chambers) => {
                let mut rtn = vec!();
                for chamber in chambers.values(){
                    rtn.append(&mut chamber.messages() );
                }
                Ok(rtn)
            }
        }
    }
}

struct TronMessagingChamber
{
    deliveries: Vec<MessageDelivery>
}

impl TronMessagingChamber
{
    pub fn new()->Self
    {
        TronMessagingChamber {
            deliveries: vec!()
        }
    }

    pub fn messages(&self)->Vec<Arc<Message>>
    {
        self.deliveries.iter().map( |d| { d.message.clone() } ).collect()
    }

    pub fn intake( &mut self, delivery: MessageDelivery )->Result<(),Box<dyn Error>>
    {
        self.deliveries.push(delivery);
        Ok(())
    }
}


pub struct NucleusPhasicMessagingStructure
{
    store: HashMap<u8,Vec<Arc<Message>>>
}

impl NucleusPhasicMessagingStructure
{
    pub fn new( ) -> Self {
        NucleusPhasicMessagingStructure {
            store: HashMap::new()
        }
    }

    pub fn intake( &mut self, message: Arc<Message> ) -> Result<(),Box<dyn Error>>
    {
       if !self.store.contains_key(&message.to.phase )
       {
           self.store.insert(message.to.phase.clone(), vec!() );
       }
       let mut messages = self.store.get( &message.to.phase ).unwrap();
       messages.push( message );

       Ok(())
    }

    pub fn remove( &mut self, phase: &u8)->Result<Option<Vec<Arc<Message>>>,Box<dyn Error>>
    {
        let option = self.store.get(phase);
        match option {
            None => Ok(Option::None),
            Some(messages) => {
                let mut rtn = vec!();
                for message in messages
                {
                    rtn.push( message.clone() );
                }
                Ok(Option::Some(rtn))
            }
        }
    }

}
